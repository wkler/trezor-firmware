# Automatically generated by pb2py
import protobuf as p


class WordAck(p.MessageType):
    MESSAGE_WIRE_TYPE = 47
    FIELDS = {
        1: ('word', p.UnicodeType, 0),  # required
    }

    def __init__(
        self,
        word: str = None
    ) -> None:
        self.word = word
