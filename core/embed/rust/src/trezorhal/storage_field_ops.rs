use crate::{
    error::Error,
    micropython::{
        map::Map,
        obj::{Obj, ObjBase},
        qstr::Qstr,
        typ::Type,
    },
};

use super::{storage::StorageResult, storage_field::Field};
use heapless::{String, Vec};

pub trait FieldOpsBase {
    fn has(&self) -> bool;
    fn delete(&self) -> StorageResult<()>;
}
impl<T> FieldOpsBase for Field<T> {
    fn has(&self) -> bool {
        self.has()
    }

    fn delete(&self) -> StorageResult<()> {
        self.delete()
    }
}

pub trait FieldGetSet<T> {
    fn get(&self) -> Option<T>;
    fn set(&self, value: T) -> StorageResult<()>;
}

pub trait FieldOps<T>: FieldOpsBase + FieldGetSet<T> {}
impl<T> FieldOps<T> for Field<T> where Field<T>: FieldGetSet<T> {}

pub struct FieldObj<T> {
    base: ObjBase,
    field: Field<T>,
    validator: Option<fn(T) -> StorageResult<T>>,
}

impl<T> FieldObj<T>
where
    Field<T>: FieldOps<T>,
    T: TryInto<Obj, Error = Error>,
{
    pub const fn from(field: Field<T>) -> Self {
        Self {
            base: Self::obj_type().as_base(),
            field,
            validator: None,
        }
    }

    fn obj_type() -> &'static Type {
        static TYPE: Type = obj_type! {
            name: Qstr::MP_QSTR_FieldType,
            locals: &obj_dict!(obj_map! {
                Qstr::MP_QSTR_get => obj_fn_1!(blabla_get),
                Qstr::MP_QSTR_set => obj_fn_2!(blabla_set),

            }),
        };
        &TYPE
    }

    pub fn obj_get(&self) -> Result<Obj, Error> {
        self.field.get().try_into()
    }
}

// So that we can call all methods directly on FieldObj
impl<T> FieldGetSet<T> for FieldObj<T>
where
    Field<T>: FieldGetSet<T>,
{
    fn get(&self) -> Option<T> {
        // TODO: allow for changing/validating the value before returning
        self.field.get()
    }

    fn set(&self, val: T) -> StorageResult<()> {
        let val = self.validator.map_or(Ok(val), |f| f(val))?;
        // TODO: allow for changing/validating the value before setting
        self.field.set(val)
    }
}
impl<T> FieldOpsBase for FieldObj<T> {
    fn has(&self) -> bool {
        self.field.has()
    }

    fn delete(&self) -> StorageResult<()> {
        self.field.delete()
    }
}

// impl<T> FieldGetSet<T> for Field<T> {
//     fn get(&self) -> Option<T> {
//         self.get()
//     }

//     fn set(&self, val: T) -> StorageResult<()> {
//         self.set(val)
//     }
// }

// So that we can define all the implementation details elsewhere
// and also use the Field directly and with other methods than get/set
// (for example set_true_or_delete() for bool)
impl FieldGetSet<u32> for Field<u32> {
    fn get(&self) -> Option<u32> {
        self.get()
    }

    fn set(&self, val: u32) -> StorageResult<()> {
        self.set(val)
    }
}
impl FieldGetSet<u16> for Field<u16> {
    fn get(&self) -> Option<u16> {
        self.get()
    }

    fn set(&self, val: u16) -> StorageResult<()> {
        self.set(val)
    }
}
impl FieldGetSet<u8> for Field<u8> {
    fn get(&self) -> Option<u8> {
        self.get()
    }

    fn set(&self, val: u8) -> StorageResult<()> {
        self.set(val)
    }
}
impl FieldGetSet<bool> for Field<bool> {
    fn get(&self) -> Option<bool> {
        self.get()
    }

    fn set(&self, val: bool) -> StorageResult<()> {
        self.set(val)
    }
}
impl<const N: usize> FieldGetSet<String<N>> for Field<String<N>> {
    fn get(&self) -> Option<String<N>> {
        self.get()
    }

    fn set(&self, val: String<N>) -> StorageResult<()> {
        self.set(val)
    }
}
// impl<const N: usize> FieldGetSet<Vec<u8, N>> for Field<Vec<u8, N>> {
//     fn get(&self) -> Option<Vec<u8, N>> {
//         self.get()
//     }

//     fn set(&self, val: Vec<u8, N>) -> StorageResult<()> {
//         self.set(val)
//     }
// }

const FIELD: Field<u32> = Field::public(0x10, 0x10);
const ABC: FieldObj<u32> = FieldObj::from(FIELD);
