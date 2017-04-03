/*
 * Copyright (c) Pavol Rusnak, SatoshiLabs
 *
 * Licensed under TREZOR License
 * see LICENSE file for details
 */

#include "py/objstr.h"

#include "trezor-crypto/rfc6979.h"

typedef struct _mp_obj_Rfc6979_t {
    mp_obj_base_t base;
    rfc6979_state rng;
} mp_obj_Rfc6979_t;

STATIC mp_obj_t mod_TrezorCrypto_Rfc6979_make_new(const mp_obj_type_t *type, size_t n_args, size_t n_kw, const mp_obj_t *args) {
    mp_arg_check_num(n_args, n_kw, 2, 2, false);
    mp_obj_Rfc6979_t *o = m_new_obj(mp_obj_Rfc6979_t);
    o->base.type = type;
    mp_buffer_info_t pkey, hash;
    mp_get_buffer_raise(args[0], &pkey, MP_BUFFER_READ);
    mp_get_buffer_raise(args[1], &hash, MP_BUFFER_READ);
    if (pkey.len != 32) {
        mp_raise_ValueError("Private key has to be 32 bytes long");
    }
    if (hash.len != 32) {
        mp_raise_ValueError("Hash has to be 32 bytes long");
    }
    init_rfc6979((const uint8_t *)pkey.buf, (const uint8_t *)hash.buf, &(o->rng));
    return MP_OBJ_FROM_PTR(o);
}

/// def trezor.crypto.rfc6979.next() -> bytes:
///     '''
///     Compute next 32-bytes of pseudorandom data
///     '''
STATIC mp_obj_t mod_TrezorCrypto_Rfc6979_next(mp_obj_t self) {
    mp_obj_Rfc6979_t *o = MP_OBJ_TO_PTR(self);
    vstr_t vstr;
    vstr_init_len(&vstr, 32);
    generate_rfc6979((uint8_t *)vstr.buf, &(o->rng));
    return mp_obj_new_str_from_vstr(&mp_type_bytes, &vstr);
}
STATIC MP_DEFINE_CONST_FUN_OBJ_1(mod_TrezorCrypto_Rfc6979_next_obj, mod_TrezorCrypto_Rfc6979_next);

STATIC const mp_rom_map_elem_t mod_TrezorCrypto_Rfc6979_locals_dict_table[] = {
    { MP_ROM_QSTR(MP_QSTR_next), MP_ROM_PTR(&mod_TrezorCrypto_Rfc6979_next_obj) },
};
STATIC MP_DEFINE_CONST_DICT(mod_TrezorCrypto_Rfc6979_locals_dict, mod_TrezorCrypto_Rfc6979_locals_dict_table);

STATIC const mp_obj_type_t mod_TrezorCrypto_Rfc6979_type = {
    { &mp_type_type },
    .name = MP_QSTR_Rfc6979,
    .make_new = mod_TrezorCrypto_Rfc6979_make_new,
    .locals_dict = (void*)&mod_TrezorCrypto_Rfc6979_locals_dict,
};
