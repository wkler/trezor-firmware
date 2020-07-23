from storage import cache, device
from trezor import wire
from trezor.crypto import bip32

from apps.common import mnemonic
from apps.common.passphrase import get as get_passphrase

from .helpers import seed_namespaces

if False:
    from apps.common.paths import Bip32Path
    from apps.common.keychain import MsgIn, MsgOut, Handler, HandlerWithKeychain


class Keychain:
    """Cardano keychain hard-coded to Byron and Shelley seed namespaces."""

    def __init__(self, root: bip32.HDNode) -> None:
        self.root = root
        self.byron_root = self._create_namespace_root(seed_namespaces.BYRON)
        self.shelley_root = self._create_namespace_root(seed_namespaces.SHELLEY)

    def verify_path(self, path: Bip32Path) -> None:
        if not is_byron_path(path) and not is_shelley_path(path):
            raise wire.DataError("Forbidden key path")

    def _create_namespace_root(self, namespace: list):
        new_root = self.root.clone()
        for i in namespace:
            new_root.derive_cardano(i)

        return new_root

    def _get_path_root(self, path: list):
        if is_byron_path(path):
            return self.byron_root
        elif is_shelley_path(path):
            return self.shelley_root
        else:
            raise wire.DataError("Forbidden key path")

    def derive(self, node_path: Bip32Path) -> bip32.HDNode:
        self.verify_path(node_path)
        path_root = self._get_path_root(node_path)

        # this is true now, so for simplicity we don't branch on path type
        assert len(seed_namespaces.BYRON) == len(seed_namespaces.SHELLEY)
        suffix = node_path[len(seed_namespaces.SHELLEY) :]

        # derive child node from the root
        node = path_root.clone()
        for i in suffix:
            node.derive_cardano(i)
        return node

    # XXX the root node remains in session cache so we should not delete it
    # def __del__(self) -> None:
    #     self.root.__del__()


def is_byron_path(path: Bip32Path):
    return path[: len(seed_namespaces.BYRON)] == seed_namespaces.BYRON


def is_shelley_path(path: Bip32Path):
    return path[: len(seed_namespaces.SHELLEY)] == seed_namespaces.SHELLEY


@cache.stored_async(cache.APP_CARDANO_ROOT)
async def get_keychain(ctx: wire.Context) -> Keychain:
    if not device.is_initialized():
        raise wire.NotInitialized("Device is not initialized")

    passphrase = await get_passphrase(ctx)
    if mnemonic.is_bip39():
        # derive the root node from mnemonic and passphrase via Cardano Icarus algorithm
        root = bip32.from_mnemonic_cardano(mnemonic.get_secret().decode(), passphrase)
    else:
        # derive the root node via SLIP-0023
        seed = mnemonic.get_seed(passphrase)
        root = bip32.from_seed(seed, "ed25519 cardano seed")

    keychain = Keychain(root)
    return keychain


def with_keychain(func: HandlerWithKeychain[MsgIn, MsgOut]) -> Handler[MsgIn, MsgOut]:
    async def wrapper(ctx: wire.Context, msg: MsgIn) -> MsgOut:
        keychain = await get_keychain(ctx)
        return await func(ctx, msg, keychain)

    return wrapper
