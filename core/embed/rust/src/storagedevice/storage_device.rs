use crate::{
    error::Error,
    micropython::{
        buffer::{Buffer, StrBuffer},
        map::Map,
        module::Module,
        obj::Obj,
        qstr::Qstr,
    },
    storagedevice::helpers,
    trezorhal::{random, storage, storage_field::Field, storage_field_ops::FieldObj},
    util,
};
use core::convert::{TryFrom, TryInto};
use cstr_core::cstr;
use heapless::{String, Vec};

const ABC: FieldObj<u32> = FieldObj::private(0x01, 0x01);

// TODO: can we import messages enums?

// TODO: would be worth to write util::try_or_raise_and_return_none which would
// take a block not returning anything and return Obj::const_none() into
// micropython? It would save some manual returns of Obj::const_none()
// Or some other way how to reduce it
// NOTE: I tried the .map() and .and_then to return it as a chain, but it did
// not work

// TODO: could be defined as a part of `Field` not to repeat it so much
const APP_DEVICE: u8 = 0x01;

const _FALSE_BYTE: u8 = 0x00;
const _TRUE_BYTE: u8 = 0x01;

const HOMESCREEN_MAXSIZE: usize = 16384;
const LABEL_MAXLENGTH: usize = 32;

// Length of SD salt auth tag.
// Other SD-salt-related constants are in core/src/storage/sd_salt.py
const SD_SALT_AUTH_KEY_LEN_BYTES: usize = 16;

// TODO: how to export all public constants as integers?

const DEVICE_ID: Field<String<24>> = Field::public(APP_DEVICE, 0x00).exact();
const VERSION: Field<Vec<u8, 1>> = Field::private(APP_DEVICE, 0x01).exact();
const _MNEMONIC_SECRET: Field<Vec<u8, 256>> = Field::private(APP_DEVICE, 0x02);
const _LABEL: Field<String<LABEL_MAXLENGTH>> = Field::public(APP_DEVICE, 0x04);
const _USE_PASSPHRASE: Field<bool> = Field::private(APP_DEVICE, 0x05);
const _HOMESCREEN: Field<Vec<u8, HOMESCREEN_MAXSIZE>> = Field::public(APP_DEVICE, 0x06);
const _NEEDS_BACKUP: Field<bool> = Field::private(APP_DEVICE, 0x07);
const _FLAGS: Field<u32> = Field::private(APP_DEVICE, 0x08);
const _U2F_COUNTER_PRIVATE: Field<u8> = Field::private(APP_DEVICE, 0x09);
const U2F_COUNTER: u8 = 0x09;
const _PASSPHRASE_ALWAYS_ON_DEVICE: Field<bool> = Field::private(APP_DEVICE, 0x0A);
const _UNFINISHED_BACKUP: Field<bool> = Field::private(APP_DEVICE, 0x0B);
const _AUTOLOCK_DELAY_MS: Field<u32> = Field::private(APP_DEVICE, 0x0C);
const _NO_BACKUP: Field<bool> = Field::private(APP_DEVICE, 0x0D);
const _BACKUP_TYPE: Field<u8> = Field::private(APP_DEVICE, 0x0E);
const ROTATION: Field<u16> = Field::public(APP_DEVICE, 0x0F);
const _SLIP39_IDENTIFIER: Field<u16> = Field::private(APP_DEVICE, 0x10);
const _SLIP39_ITERATION_EXPONENT: Field<u8> = Field::private(APP_DEVICE, 0x11);
const _SD_SALT_AUTH_KEY: Field<Vec<u8, SD_SALT_AUTH_KEY_LEN_BYTES>> =
    Field::public(APP_DEVICE, 0x12).exact();
const INITIALIZED: Field<bool> = Field::public(APP_DEVICE, 0x13);
const _SAFETY_CHECK_LEVEL: Field<u8> = Field::private(APP_DEVICE, 0x14);
const _EXPERIMENTAL_FEATURES: Field<bool> = Field::private(APP_DEVICE, 0x15);

const SAFETY_CHECK_LEVEL_STRICT: u8 = 0;
const SAFETY_CHECK_LEVEL_PROMPT: u8 = 1;
const _DEFAULT_SAFETY_CHECK_LEVEL: u8 = SAFETY_CHECK_LEVEL_STRICT;
const SAFETY_CHECK_LEVELS: [u8; 2] = [SAFETY_CHECK_LEVEL_STRICT, SAFETY_CHECK_LEVEL_PROMPT];

// TODO: somehow determine the DEBUG_MODE value
const DEBUG_MODE: bool = true;
const AUTOLOCK_DELAY_DEFAULT: u32 = 10 * 60 * 1000; // 10 minutes
const AUTOLOCK_DELAY_MINIMUM: u32 = if DEBUG_MODE {
    10 * 1000 // 10 seconds
} else {
    60 * 1000 // 1 minute
};
// autolock intervals larger than AUTOLOCK_DELAY_MAXIMUM cause issues in the
// scheduler
const AUTOLOCK_DELAY_MAXIMUM: u32 = 0x2000_0000; // ~6 days

const STORAGE_VERSION_CURRENT: u8 = 0x02;

extern "C" fn storagedevice_is_version_stored() -> Obj {
    let block = || Ok(VERSION.has().into());
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_get_version() -> Obj {
    let block = || {
        if let Some(result) = VERSION.get() {
            (&result as &[u8]).try_into()
        } else {
            Ok(Obj::const_none())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_version(version: Obj) -> Obj {
    let block = || {
        VERSION.set(Buffer::try_from(version)?.as_ref())?;
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_is_initialized() -> Obj {
    let block = || {
        if let Some(result) = INITIALIZED.get() {
            Ok(result.into())
        } else {
            Ok(false.into())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_is_initialized(is_initialized: Obj) -> Obj {
    let block = || {
        INITIALIZED.set(bool::try_from(is_initialized)?)?;
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_get_rotation() -> Obj {
    let block = || {
        // 0 is a default when not there
        Ok(ROTATION.get().unwrap_or(0).into())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_rotation(rotation: Obj) -> Obj {
    let block = || {
        let rotation = u16::try_from(rotation)?;

        if ![0, 90, 180, 270].contains(&rotation) {
            Err(Error::ValueError(cstr!("Not valid rotation")))
        } else {
            ROTATION.set(rotation)?;
            Ok(Obj::const_none())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_get_label() -> Obj {
    let block = || {
        if let Some(result) = _LABEL.get() {
            result.as_str().try_into()
        } else {
            Ok(Obj::const_none())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_label(label: Obj) -> Obj {
    let block = || {
        let label = StrBuffer::try_from(label)?;
        _LABEL.set(String::from(label.as_ref()))?;
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_get_device_id() -> Obj {
    let block = || {
        if let Some(device_id) = DEVICE_ID.get() {
            device_id.as_str().try_into()
        } else {
            let new_device_id = &random::get_random_bytes(12) as &[u8];
            let hex_id = helpers::hexlify_bytes(new_device_id);
            DEVICE_ID.set(String::from(hex_id.as_str()))?;
            hex_id.as_str().try_into()
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_device_id(device_id: Obj) -> Obj {
    let block = || {
        let device_id = StrBuffer::try_from(device_id)?;
        DEVICE_ID.set(String::from(device_id.as_ref()))?;
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_get_mnemonic_secret() -> Obj {
    let block = || {
        if let Some(result) = _MNEMONIC_SECRET.get() {
            (&result as &[u8]).try_into()
        } else {
            Ok(Obj::const_none())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_mnemonic_secret(
    n_args: usize,
    args: *const Obj,
    kwargs: *mut Map,
) -> Obj {
    let block = |_args: &[Obj], kwargs: &Map| {
        let secret: Buffer = kwargs.get(Qstr::MP_QSTR_secret)?.try_into()?;
        let backup_type: u8 = kwargs.get(Qstr::MP_QSTR_backup_type)?.try_into()?;
        let needs_backup: bool = if kwargs.contains_key(Qstr::MP_QSTR_needs_backup) {
            kwargs.get(Qstr::MP_QSTR_needs_backup)?.try_into()?
        } else {
            false
        };
        let no_backup: bool = if kwargs.contains_key(Qstr::MP_QSTR_no_backup) {
            kwargs.get(Qstr::MP_QSTR_no_backup)?.try_into()?
        } else {
            false
        };

        VERSION.set(&[STORAGE_VERSION_CURRENT])?;
        _MNEMONIC_SECRET.set(secret.as_ref())?;
        _BACKUP_TYPE.set(backup_type)?;
        _NO_BACKUP.set_true_or_delete(no_backup)?;
        INITIALIZED.set(true)?;

        if !no_backup {
            _NEEDS_BACKUP.set_true_or_delete(needs_backup)?;
        }

        Ok(true.into())
    };
    unsafe { util::try_with_args_and_kwargs(n_args, args, kwargs, block) }
}

extern "C" fn storagedevice_is_passphrase_enabled() -> Obj {
    let block = || {
        if let Some(result) = _USE_PASSPHRASE.get() {
            Ok(result.into())
        } else {
            Ok(false.into())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_passphrase_enabled(enable: Obj) -> Obj {
    let block = || {
        let enable = bool::try_from(enable)?;

        _USE_PASSPHRASE.set(enable)?;

        if !enable {
            _PASSPHRASE_ALWAYS_ON_DEVICE.set(false)?;
        }

        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_get_passphrase_always_on_device() -> Obj {
    let block = || {
        if let Some(result) = _PASSPHRASE_ALWAYS_ON_DEVICE.get() {
            Ok(result.into())
        } else {
            Ok(false.into())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_passphrase_always_on_device(enable: Obj) -> Obj {
    let block = || {
        _PASSPHRASE_ALWAYS_ON_DEVICE.set(bool::try_from(enable)?)?;
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_unfinished_backup() -> Obj {
    let block = || {
        if let Some(result) = _UNFINISHED_BACKUP.get() {
            Ok(result.into())
        } else {
            Ok(false.into())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_unfinished_backup(state: Obj) -> Obj {
    let block = || {
        _UNFINISHED_BACKUP.set(bool::try_from(state)?)?;
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_needs_backup() -> Obj {
    let block = || {
        if let Some(result) = _NEEDS_BACKUP.get() {
            Ok(result.into())
        } else {
            Ok(false.into())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_backed_up() -> Obj {
    let block = || {
        _NEEDS_BACKUP.delete()?;
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_no_backup() -> Obj {
    let block = || {
        if let Some(result) = _NO_BACKUP.get() {
            Ok(result.into())
        } else {
            Ok(false.into())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_get_experimental_features() -> Obj {
    let block = || {
        if let Some(result) = _EXPERIMENTAL_FEATURES.get() {
            Ok(result.into())
        } else {
            Ok(false.into())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_experimental_features(enable: Obj) -> Obj {
    let block = || {
        let enable = bool::try_from(enable)?;
        _EXPERIMENTAL_FEATURES.set_true_or_delete(enable)?;
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_get_backup_type() -> Obj {
    let block = || {
        // TODO: we could import the BackupType enum
        let backup_type = _BACKUP_TYPE.get().unwrap_or(0);

        if ![0, 1, 2].contains(&backup_type) {
            Err(Error::ValueError(cstr!("Invalid backup type")))
        } else {
            Ok(backup_type.into())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_get_homescreen() -> Obj {
    let block = || {
        if let Some(result) = _HOMESCREEN.get() {
            (&result as &[u8]).try_into()
        } else {
            Ok(Obj::const_none())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_homescreen(homescreen: Obj) -> Obj {
    let block = || {
        _HOMESCREEN.set(Buffer::try_from(homescreen)?.as_ref())?;
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_get_slip39_identifier() -> Obj {
    let block = || {
        if let Some(result) = _SLIP39_IDENTIFIER.get() {
            Ok(result.into())
        } else {
            Ok(Obj::const_none())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_slip39_identifier(identifier: Obj) -> Obj {
    let block = || {
        _SLIP39_IDENTIFIER.set(u16::try_from(identifier)?)?;
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_get_slip39_iteration_exponent() -> Obj {
    let block = || {
        if let Some(result) = _SLIP39_ITERATION_EXPONENT.get() {
            Ok(result.into())
        } else {
            Ok(Obj::const_none())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_slip39_iteration_exponent(exponent: Obj) -> Obj {
    let block = || {
        _SLIP39_ITERATION_EXPONENT.set(u8::try_from(exponent)?)?;
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_get_autolock_delay_ms() -> Obj {
    let block = || {
        let delay = _AUTOLOCK_DELAY_MS.get().unwrap_or(AUTOLOCK_DELAY_DEFAULT);
        _normalize_autolock_delay(delay).try_into()
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_autolock_delay_ms(delay_ms: Obj) -> Obj {
    let block = || {
        _AUTOLOCK_DELAY_MS.set(u32::try_from(delay_ms)?)?;
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_get_flags() -> Obj {
    let block = || _FLAGS.get().unwrap_or(0).try_into();
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_flags(flags: Obj) -> Obj {
    let block = || {
        let flags = u32::try_from(flags)?;

        let old_flags = _FLAGS.get().unwrap_or(0);

        // Not deleting old flags
        let new_flags = flags | old_flags;
        _FLAGS.set(new_flags)?;
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_get_safety_check_level() -> Obj {
    let block = || {
        let level = _SAFETY_CHECK_LEVEL
            .get()
            .unwrap_or(_DEFAULT_SAFETY_CHECK_LEVEL);

        let level = if !SAFETY_CHECK_LEVELS.contains(&level) {
            _DEFAULT_SAFETY_CHECK_LEVEL
        } else {
            level
        };

        Ok(level.into())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_safety_check_level(level: Obj) -> Obj {
    let block = || {
        let level = u8::try_from(level)?;

        if !SAFETY_CHECK_LEVELS.contains(&level) {
            return Err(Error::ValueError(cstr!("Not valid safety level")));
        } else {
            _SAFETY_CHECK_LEVEL.set(level)?;
            Ok(Obj::const_none())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_get_sd_salt_auth_key() -> Obj {
    let block = || {
        if let Some(result) = _SD_SALT_AUTH_KEY.get() {
            (&result as &[u8]).try_into()
        } else {
            Ok(Obj::const_none())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_sd_salt_auth_key(auth_key: Obj) -> Obj {
    let block = || {
        let auth_key = Buffer::try_from(auth_key)?;

        if auth_key.is_empty() {
            _SD_SALT_AUTH_KEY.delete()?
        } else {
            _SD_SALT_AUTH_KEY.set(auth_key.as_ref())?
        }
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_get_next_u2f_counter() -> Obj {
    let block = || {
        let key = helpers::get_appkey_u2f(APP_DEVICE, U2F_COUNTER, true);
        storage::get_next_counter(key)?.try_into()
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_set_u2f_counter(count: Obj) -> Obj {
    let block = || {
        let count = u32::try_from(count)?;

        let key = helpers::get_appkey_u2f(APP_DEVICE, U2F_COUNTER, true);
        storage::set_counter(key, count)?;
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_get_private_u2f_counter() -> Obj {
    let block = || {
        if let Some(result) = _U2F_COUNTER_PRIVATE.get() {
            Ok(result.into())
        } else {
            Ok(Obj::const_none())
        }
    };
    unsafe { util::try_or_raise(block) }
}

extern "C" fn storagedevice_delete_private_u2f_counter() -> Obj {
    let block = || {
        _EXPERIMENTAL_FEATURES.delete()?;
        Ok(Obj::const_none())
    };
    unsafe { util::try_or_raise(block) }
}

fn _normalize_autolock_delay(delay_ms: u32) -> u32 {
    if delay_ms < AUTOLOCK_DELAY_MINIMUM {
        AUTOLOCK_DELAY_MINIMUM
    } else if delay_ms > AUTOLOCK_DELAY_MAXIMUM {
        AUTOLOCK_DELAY_MAXIMUM
    } else {
        delay_ms
    }
}

#[no_mangle]
pub static mp_module_trezorstoragedevice: Module = obj_module! {
    Qstr::MP_QSTR___name_storage__ => Qstr::MP_QSTR_trezorstoragedevice.to_obj(),

    /// BackupTypeInt = TypeVar("BackupTypeInt", bound=int)
    /// StorageSafetyCheckLevel = Literal[0, 1]

    /// def is_version_stored() -> bool:
    ///     """Whether version is in storage."""
    Qstr::MP_QSTR_is_version_stored => obj_fn_0!(storagedevice_is_version_stored).as_obj(),

    /// def is_initialized() -> bool:
    ///     """Whether device is initialized."""
    Qstr::MP_QSTR_is_initialized => obj_fn_0!(storagedevice_is_initialized).as_obj(),

    /// def set_is_initialized(is_initialized: bool) -> None:
    ///     """Whether device is initialized."""
    Qstr::MP_QSTR_set_is_initialized => obj_fn_1!(storagedevice_set_is_initialized).as_obj(),

    /// def get_version() -> bytes | None:
    ///     """Get version."""
    Qstr::MP_QSTR_get_version => obj_fn_0!(storagedevice_get_version).as_obj(),

    /// def set_version(version: bytes) -> None:
    ///     """Set version."""
    Qstr::MP_QSTR_set_version => obj_fn_1!(storagedevice_set_version).as_obj(),

    /// def get_rotation() -> int:
    ///     """Get rotation."""
    Qstr::MP_QSTR_get_rotation => obj_fn_0!(storagedevice_get_rotation).as_obj(),

    /// def set_rotation(rotation: int) -> None:
    ///     """Set rotation."""
    Qstr::MP_QSTR_set_rotation => obj_fn_1!(storagedevice_set_rotation).as_obj(),

    /// def get_label() -> str | None:
    ///     """Get label."""
    Qstr::MP_QSTR_get_label => obj_fn_0!(storagedevice_get_label).as_obj(),

    /// def set_label(label: str) -> None:
    ///     """Set label."""
    Qstr::MP_QSTR_set_label => obj_fn_1!(storagedevice_set_label).as_obj(),

    /// def get_device_id() -> str:
    ///     """Get device ID."""
    Qstr::MP_QSTR_get_device_id => obj_fn_0!(storagedevice_get_device_id).as_obj(),

    /// def set_device_id(device_id: str) -> None:
    ///     """Set device ID."""
    Qstr::MP_QSTR_set_device_id => obj_fn_1!(storagedevice_set_device_id).as_obj(),

    /// def get_mnemonic_secret() -> bytes | None:
    ///     """Get mnemonic secret."""
    Qstr::MP_QSTR_get_mnemonic_secret => obj_fn_0!(storagedevice_get_mnemonic_secret).as_obj(),

    /// def set_mnemonic_secret(
    ///     *,
    ///     secret: bytes,
    ///     backup_type: BackupTypeInt,
    ///     needs_backup: bool = False,
    ///     no_backup: bool = False,
    /// ) -> None:
    ///     """Set mnemonic secret. Only kwargs are supported."""
    Qstr::MP_QSTR_set_mnemonic_secret => obj_fn_kw!(0, storagedevice_set_mnemonic_secret).as_obj(),

    /// def is_passphrase_enabled() -> bool:
    ///     """Whether passphrase is enabled."""
    Qstr::MP_QSTR_is_passphrase_enabled => obj_fn_0!(storagedevice_is_passphrase_enabled).as_obj(),

    /// def set_passphrase_enabled(enable: bool) -> None:
    ///     """Set whether passphrase is enabled."""
    Qstr::MP_QSTR_set_passphrase_enabled => obj_fn_1!(storagedevice_set_passphrase_enabled).as_obj(),

    /// def get_passphrase_always_on_device() -> bool:
    ///     """Whether passphrase is on device."""
    Qstr::MP_QSTR_get_passphrase_always_on_device => obj_fn_0!(storagedevice_get_passphrase_always_on_device).as_obj(),

    /// def set_passphrase_always_on_device(enable: bool) -> None:
    ///     """Set whether passphrase is on device.
    ///
    ///     This is backwards compatible with _PASSPHRASE_SOURCE:
    ///     - If ASK(0) => returns False, the check against b"\x01" in get_bool fails.
    ///     - If DEVICE(1) => returns True, the check against b"\x01" in get_bool succeeds.
    ///     - If HOST(2) => returns False, the check against b"\x01" in get_bool fails.
    ///     """
    Qstr::MP_QSTR_set_passphrase_always_on_device => obj_fn_1!(storagedevice_set_passphrase_always_on_device).as_obj(),

    /// def unfinished_backup() -> bool:
    ///     """Whether backup is still in progress."""
    Qstr::MP_QSTR_unfinished_backup => obj_fn_0!(storagedevice_unfinished_backup).as_obj(),

    /// def set_unfinished_backup(state: bool) -> None:
    ///     """Set backup state."""
    Qstr::MP_QSTR_set_unfinished_backup => obj_fn_1!(storagedevice_set_unfinished_backup).as_obj(),

    /// def needs_backup() -> bool:
    ///     """Whether backup is needed."""
    Qstr::MP_QSTR_needs_backup => obj_fn_0!(storagedevice_needs_backup).as_obj(),

    /// def set_backed_up() -> None:
    ///     """Signal that backup is finished."""
    Qstr::MP_QSTR_set_backed_up => obj_fn_0!(storagedevice_set_backed_up).as_obj(),

    /// def no_backup() -> bool:
    ///     """Whether there is no backup."""
    Qstr::MP_QSTR_no_backup => obj_fn_0!(storagedevice_no_backup).as_obj(),

    /// def get_backup_type() -> BackupTypeInt:
    ///     """Get backup type."""
    Qstr::MP_QSTR_get_backup_type => obj_fn_0!(storagedevice_get_backup_type).as_obj(),

    /// def get_homescreen() -> bytes | None:
    ///     """Get homescreen."""
    Qstr::MP_QSTR_get_homescreen => obj_fn_0!(storagedevice_get_homescreen).as_obj(),

    /// def set_homescreen(homescreen: bytes) -> None:
    ///     """Set homescreen."""
    Qstr::MP_QSTR_set_homescreen => obj_fn_1!(storagedevice_set_homescreen).as_obj(),

    /// def get_slip39_identifier() -> int | None:
    ///     """The device's actual SLIP-39 identifier used in passphrase derivation."""
    Qstr::MP_QSTR_get_slip39_identifier => obj_fn_0!(storagedevice_get_slip39_identifier).as_obj(),

    /// def set_slip39_identifier(identifier: int) -> None:
    ///     """
    ///     The device's actual SLIP-39 identifier used in passphrase derivation.
    ///     Not to be confused with recovery.identifier, which is stored only during
    ///     the recovery process and it is copied here upon success.
    ///     """
    Qstr::MP_QSTR_set_slip39_identifier => obj_fn_1!(storagedevice_set_slip39_identifier).as_obj(),

    /// def get_slip39_iteration_exponent() -> int | None:
    ///     """The device's actual SLIP-39 iteration exponent used in passphrase derivation."""
    Qstr::MP_QSTR_get_slip39_iteration_exponent => obj_fn_0!(storagedevice_get_slip39_iteration_exponent).as_obj(),

    /// def set_slip39_iteration_exponent(exponent: int) -> None:
    ///     """
    ///     The device's actual SLIP-39 iteration exponent used in passphrase derivation.
    ///     Not to be confused with recovery.iteration_exponent, which is stored only during
    ///     the recovery process and it is copied here upon success.
    ///     """
    Qstr::MP_QSTR_set_slip39_iteration_exponent => obj_fn_1!(storagedevice_set_slip39_iteration_exponent).as_obj(),

    /// def get_autolock_delay_ms() -> int:
    ///     """Get autolock delay."""
    Qstr::MP_QSTR_get_autolock_delay_ms => obj_fn_0!(storagedevice_get_autolock_delay_ms).as_obj(),

    /// def set_autolock_delay_ms(delay_ms: int) -> None:
    ///     """Set autolock delay."""
    Qstr::MP_QSTR_set_autolock_delay_ms => obj_fn_1!(storagedevice_set_autolock_delay_ms).as_obj(),

    /// def get_flags() -> int:
    ///     """Get flags."""
    Qstr::MP_QSTR_get_flags => obj_fn_0!(storagedevice_get_flags).as_obj(),

    /// def set_flags(flags: int) -> None:
    ///     """Set flags."""
    Qstr::MP_QSTR_set_flags => obj_fn_1!(storagedevice_set_flags).as_obj(),

    /// def get_safety_check_level() -> StorageSafetyCheckLevel:
    ///     """Get safety check level.
    ///     Do not use this function directly, see apps.common.safety_checks instead.
    ///     """
    Qstr::MP_QSTR_get_safety_check_level => obj_fn_0!(storagedevice_get_safety_check_level).as_obj(),

    /// def set_safety_check_level(level: StorageSafetyCheckLevel) -> None:
    ///     """Set safety check level.
    ///     Do not use this function directly, see apps.common.safety_checks instead.
    ///     """
    Qstr::MP_QSTR_set_safety_check_level => obj_fn_1!(storagedevice_set_safety_check_level).as_obj(),

    /// def get_sd_salt_auth_key() -> bytes | None:
    ///     """The key used to check the authenticity of the SD card salt."""
    Qstr::MP_QSTR_get_sd_salt_auth_key => obj_fn_0!(storagedevice_get_sd_salt_auth_key).as_obj(),

    /// def set_sd_salt_auth_key(auth_key: bytes) -> None:
    ///     """The key used to check the authenticity of the SD card salt.
    ///     Empty bytes will delete the salt.
    ///     """
    Qstr::MP_QSTR_set_sd_salt_auth_key => obj_fn_1!(storagedevice_set_sd_salt_auth_key).as_obj(),

    /// def get_next_u2f_counter() -> int:
    ///     """Get next U2F counter."""
    Qstr::MP_QSTR_get_next_u2f_counter => obj_fn_0!(storagedevice_get_next_u2f_counter).as_obj(),

    /// def set_u2f_counter(count: int) -> None:
    ///     """Set U2F counter."""
    Qstr::MP_QSTR_set_u2f_counter => obj_fn_1!(storagedevice_set_u2f_counter).as_obj(),

    /// def get_private_u2f_counter() -> int | None:
    ///     """Get private U2F counter."""
    Qstr::MP_QSTR_get_private_u2f_counter => obj_fn_0!(storagedevice_get_private_u2f_counter).as_obj(),

    /// def delete_private_u2f_counter() -> None:
    ///     """Delete private U2F counter."""
    Qstr::MP_QSTR_delete_private_u2f_counter => obj_fn_0!(storagedevice_delete_private_u2f_counter).as_obj(),

    /// def get_experimental_features() -> bool:
    ///     """Whether we have experimental features."""
    Qstr::MP_QSTR_get_experimental_features => obj_fn_0!(storagedevice_get_experimental_features).as_obj(),

    /// def set_experimental_features(enabled: bool) -> None:
    ///     """Set experimental features."""
    Qstr::MP_QSTR_set_experimental_features => obj_fn_1!(storagedevice_set_experimental_features).as_obj(),

    // Qstr::MP_QSTR_set_experimental_features => obj_fn_0!(ABC).as_obj(),
    // Qstr::MP_QSTR_set_experimental_features => obj_type!(ABC).as_obj(),
    // Qstr::MP_QSTR_set_experimental_features => obj_map!(ABC).as_obj(),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_autolock_delay_small() {
        let result = _normalize_autolock_delay(123);
        assert_eq!(result, AUTOLOCK_DELAY_MINIMUM);
    }

    #[test]
    fn normalize_autolock_delay_big() {
        let result = _normalize_autolock_delay(u32::MAX);
        assert_eq!(result, AUTOLOCK_DELAY_MAXIMUM);
    }

    #[test]
    fn normalize_autolock_delay_normal() {
        let result = _normalize_autolock_delay(1_000_000);
        assert_eq!(result, 1_000_000);
    }
}
