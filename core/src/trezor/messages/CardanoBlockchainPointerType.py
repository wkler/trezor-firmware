# Automatically generated by pb2py
# fmt: off
import protobuf as p

if __debug__:
    try:
        from typing import Dict, List  # noqa: F401
        from typing_extensions import Literal  # noqa: F401
    except ImportError:
        pass


class CardanoBlockchainPointerType(p.MessageType):

    def __init__(
        self,
        block_index: int = None,
        tx_index: int = None,
        certificate_index: int = None,
    ) -> None:
        self.block_index = block_index
        self.tx_index = tx_index
        self.certificate_index = certificate_index

    @classmethod
    def get_fields(cls) -> Dict:
        return {
            1: ('block_index', p.UVarintType, 0),
            2: ('tx_index', p.UVarintType, 0),
            3: ('certificate_index', p.UVarintType, 0),
        }
