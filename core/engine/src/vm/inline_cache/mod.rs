use arrayvec::ArrayVec;
use itertools::Itertools;
use std::{cell::Cell, fmt};

use boa_gc::GcRefCell;
use boa_macros::{Finalize, Trace};

use crate::{
    JsString,
    object::{
        PropertyMap,
        shape::{
            Shape, WeakShape,
            slot::{Slot, SlotAttributes},
        },
    },
    property::PropertyKey,
};

#[cfg(test)]
mod tests;

pub(crate) const PIC_CAPACITY: usize = 4;

/// A cached shape-to-slot mapping for a polymorphic inline cache.
#[derive(Clone, Debug, Trace, Finalize)]
pub(crate) struct CacheEntry {
    /// A weak reference is kept to the shape to avoid the shape preventing deallocation.
    pub(crate) shape: WeakShape,
    #[unsafe_ignore_trace]
    pub(crate) slot: Slot,
    /// For a property found on the prototype, the shape that prototype (the holder) had when the
    /// entry was created.
    holder: Option<HolderShape>,
}

/// The shape of the prototype that holds a cached property.
#[derive(Clone, Debug, Trace, Finalize)]
struct HolderShape {
    /// A weak reference, like [`CacheEntry::shape`].
    shape: WeakShape,
    /// The address of `shape`, compared on lookup so that a hit does not have to upgrade `shape`.
    #[unsafe_ignore_trace]
    address: usize,
}

impl HolderShape {
    fn new(shape: &Shape) -> Self {
        Self {
            shape: shape.into(),
            address: shape.to_addr_usize(),
        }
    }

    /// Returns `true` if `shape` is the recorded shape.
    ///
    /// A shape allocated at the address of the recorded one after it was collected has the same
    /// address, but then the weak reference is no longer alive: the collector clears it before it
    /// frees the shape.
    fn is(&self, shape: &Shape) -> bool {
        self.address == shape.to_addr_usize() && self.shape.is_alive()
    }
}

/// An inline cache entry for a property access.
#[derive(Clone, Debug, Trace, Finalize)]
pub(crate) struct InlineCache {
    /// The property that is accessed.
    pub(crate) name: JsString,

    /// Multiple cached shape-to-slot entries.
    pub(crate) entries: GcRefCell<ArrayVec<CacheEntry, PIC_CAPACITY>>,

    /// Whether this access site has seen too many shapes and should no longer be cached.
    #[unsafe_ignore_trace]
    pub(crate) megamorphic: Cell<bool>,
}

impl fmt::Display for InlineCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(name:{} entries:", self.name.display_escaped())?;

        if self.megamorphic.get() {
            return write!(f, "(megamorphic))");
        }

        let entries = self.entries.borrow();
        let entries = entries.iter().map(|e| e.shape.to_addr_usize()).format(", ");

        write!(f, "({entries:#x}))")
    }
}

impl InlineCache {
    pub(crate) fn new(name: JsString) -> Self {
        Self {
            name,
            entries: GcRefCell::new(ArrayVec::new()),
            megamorphic: Cell::new(false),
        }
    }

    /// Caches `slot`, found by a lookup of this cache's property on an object with `shape`.
    ///
    /// The slot is only cached if the current shapes still lead to it: a getter or setter called
    /// by the lookup may have changed the objects after the slot was found.
    pub(crate) fn set(&self, shape: &Shape, slot: Slot) {
        if self.megamorphic.get() {
            return;
        }

        let key = PropertyKey::from(self.name.clone());
        let found = Some(Slot {
            index: slot.index,
            attributes: slot
                .attributes
                .difference(SlotAttributes::INLINE_CACHE_BITS),
        });
        let holder = if slot.attributes.contains(SlotAttributes::PROTOTYPE) {
            // A unique shape gains properties in place, so an object with a unique shape could
            // come to own the property without changing its shape.
            if shape.is_unique() || shape.lookup(&key).is_some() {
                return;
            }
            let Some(prototype) = shape.prototype() else {
                return;
            };
            let prototype = prototype.borrow();
            if prototype.shape().lookup(&key) != found {
                return;
            }
            Some(HolderShape::new(prototype.shape()))
        } else {
            if shape.lookup(&key) != found {
                return;
            }
            None
        };

        let mut entries = self.entries.borrow_mut();

        // Add a new entry if there's space.
        if entries
            .try_push(CacheEntry {
                shape: shape.into(),
                slot,
                holder,
            })
            .is_err()
        {
            // Polymorphic cache is full, transition to megamorphic.
            self.megamorphic.set(true);
            entries.clear();
        }
    }

    /// Returns the cached [`Slot`] for an object with the properties `properties`, together with
    /// what `read` returns for the properties that hold the slot: the object's, or its prototype's
    /// for a property found on the prototype.
    ///
    /// An entry is the shape of an object, the slot of the property and, for a property found on
    /// the prototype, the shape of that prototype (the holder). It is used only while the object
    /// has the entry's shape and the holder has the holder's shape. That keeps the slot and its
    /// attributes correct because a shape determines its object's prototype and the slots and
    /// attributes of its own properties: shared shapes never change, a unique shape changes in
    /// place only when it gains a property, which moves no slot and changes no attributes, and
    /// every other change gives the object a new shape. A property gained in place can shadow one
    /// of the prototype, so properties found on the prototype are not cached for objects with
    /// unique shapes. Shapes are held weakly, and a shape allocated at the address of a collected
    /// one does not match.
    ///
    /// Opportunistically cleans up stale weak shape references and outdated holder shapes during
    /// lookup.
    pub(crate) fn get<R>(
        &self,
        properties: &PropertyMap,
        read: impl FnOnce(&PropertyMap, Slot) -> R,
    ) -> Option<(Slot, R)> {
        if self.megamorphic.get() {
            return None;
        }

        let mut entries = self.entries.borrow_mut();
        let mut i = 0;
        let shape_addr = properties.shape.to_addr_usize();

        while i < entries.len() {
            let Some(shape) = entries[i].shape.upgrade() else {
                // Opportunistically clean up stale weak shapes.
                entries.swap_remove(i);
                continue;
            };
            if shape.to_addr_usize() != shape_addr {
                i += 1;
                continue;
            }
            let slot = entries[i].slot;
            let Some(holder) = &entries[i].holder else {
                return Some((slot, read(properties, slot)));
            };
            let prototype = shape.prototype().expect("prototype should have value");
            let prototype = prototype.borrow();
            if holder.is(prototype.shape()) {
                return Some((slot, read(prototype.properties(), slot)));
            }
            // The holder changed since the entry was created.
            entries.swap_remove(i);
            return None;
        }

        None
    }
}
