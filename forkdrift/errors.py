"""Error types for forkdrift."""


class ForkdriftError(Exception):
    """Base class for every refusal forkdrift raises."""


class ArtifactError(ForkdriftError):
    """A recorded artifact is malformed."""


class InterfaceMismatchError(ForkdriftError):
    """Two artifacts name different interfaces and cannot be compared."""
