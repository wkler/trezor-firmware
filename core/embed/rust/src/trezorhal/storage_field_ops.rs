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
    field: Field<T>,
}

impl<T> FieldObj<T> {
    pub const fn new(field: Field<T>) -> Self {
        Self { field }
    }

    // Custom constructors, so we do not need to create Field manually
    pub const fn public(app: u8, key: u8) -> Self {
        Self::new(Field::public(app, key))
    }
    pub const fn public_writable(app: u8, key: u8) -> Self {
        Self::new(Field::public_writable(app, key))
    }
    pub const fn private(app: u8, key: u8) -> Self {
        Self::new(Field::private(app, key))
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

const ABC: FieldObj<u32> = FieldObj {
    field: Field::private(0x01, 0x01),
};
const DEF: FieldObj<u32> = FieldObj::private(0x01, 0x01);

fn trial() {
    let _a = ABC.get();
    let _a = ABC.field.get();
    let _b = ABC.field.set(0x01);
    let _c = DEF.has();
    let _c = DEF.field.has();
    let _d = DEF.field.delete();
    let _d = ABC.delete();
}
