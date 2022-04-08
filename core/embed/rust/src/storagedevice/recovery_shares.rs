use crate::{
    error::Error,
    micropython::{buffer::StrBuffer, map::Map, module::Module, obj::Obj, qstr::Qstr},
    trezorhal::storage_field::Field,
    util,
};
use core::convert::TryFrom;
use heapless::{String, Vec};

const APP_RECOVERY_SHARES: u8 = 0x03;

const MAX_SHARE_COUNT: usize = 16;
const MAX_GROUP_COUNT: usize = 16;

extern "C" fn storagerecoveryshares_get(index: Obj, group_index: Obj) -> Obj {
    let block = || {
        let index = u8::try_from(index)?;
        let group_index = u8::try_from(group_index)?;

        get_share_string(index, group_index)?.as_str().try_into()
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagerecoveryshares_set(index: Obj, group_index: Obj, mnemonic: Obj) -> Obj {
    let block = || {
        let index = u8::try_from(index)?;
        let group_index = u8::try_from(group_index)?;
        let mnemonic = StrBuffer::try_from(mnemonic)?;

        Field::<String<256>>::private(
            APP_RECOVERY_SHARES,
            index + group_index * MAX_SHARE_COUNT as u8,
        )
        .set(String::from(mnemonic.as_ref()))?;
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagerecoveryshares_fetch_group(group_index: Obj) -> Obj {
    let block = || {
        let group_index = u8::try_from(group_index)?;

        let mut result: Vec<String<256>, MAX_SHARE_COUNT> = Vec::new();
        for index in 0..MAX_SHARE_COUNT {
            let share = get_share_string(index as u8, group_index)?;
            if !share.is_empty() {
                result.push(share).unwrap();
            }
        }
        result.try_into()
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagerecoveryshares_delete() -> Obj {
    let block = || {
        delete_all_recovery_shares()?;
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

pub fn get_share_string(index: u8, group_index: u8) -> Result<String<256>, Error> {
    Ok(Field::<String<256>>::private(
        APP_RECOVERY_SHARES,
        index + group_index * MAX_SHARE_COUNT as u8,
    )
    .get()
    .unwrap_or_else(|| String::from("")))
}

pub fn delete_all_recovery_shares() -> Result<(), Error> {
    for index in 0..MAX_SHARE_COUNT * MAX_GROUP_COUNT {
        Field::<String<256>>::private(APP_RECOVERY_SHARES, index as u8).delete()?;
    }
    Ok(())
}

#[no_mangle]
pub static mp_module_trezorstoragerecoveryshares: Module = obj_module! {
    Qstr::MP_QSTR___name_recoveryshares__ => Qstr::MP_QSTR_trezorstoragerecoveryshares.to_obj(),

    /// def get(index: int, group_index: int) -> str | None:
    ///     """Get recovery share."""
    Qstr::MP_QSTR_get => obj_fn_2!(storagerecoveryshares_get).as_obj(),

    /// def set(index: int, group_index: int, mnemonic: str) -> None:
    ///     """Set recovery share."""
    Qstr::MP_QSTR_set => obj_fn_3!(storagerecoveryshares_set).as_obj(),

    /// def fetch_group(group_index: int) -> list[str]:
    ///     """Fetch recovery share group."""
    Qstr::MP_QSTR_fetch_group => obj_fn_1!(storagerecoveryshares_fetch_group).as_obj(),

    /// def delete() -> None:
    ///     """Delete all recovery shares."""
    Qstr::MP_QSTR_delete => obj_fn_0!(storagerecoveryshares_delete).as_obj(),
};
