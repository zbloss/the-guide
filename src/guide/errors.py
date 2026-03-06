class GuideError(Exception):
    """Base error for The Guide."""


class NotFoundError(GuideError):
    def __init__(self, resource: str) -> None:
        super().__init__(f"{resource} not found")
        self.resource = resource


class InvalidInputError(GuideError):
    pass


class DatabaseError(GuideError):
    pass


class LlmError(GuideError):
    pass


class SerializationError(GuideError):
    pass
