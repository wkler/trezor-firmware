# Automatically generated by pb2py
import protobuf as p


class PassphraseAck(p.MessageType):
    MESSAGE_WIRE_TYPE = 42
    FIELDS = {
        1: ('passphrase', p.UnicodeType, 0),
        2: ('state', p.BytesType, 0),
    }

    def __init__(
        self,
        passphrase: str = None,
        state: bytes = None
    ) -> None:
        self.passphrase = passphrase
        self.state = state
