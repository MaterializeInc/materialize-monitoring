"""Sugar around dashboard v2 builders."""

from __future__ import annotations

import contextlib
import typing
from collections.abc import Callable


class VirtualKind(typing.Protocol):
    spec: typing.Any

    def __init__(self, spec=None): ...

    # HACK: literal string requires an immutable (property satisfies this)
    @property
    def kind(self) -> str: ...


KindT = typing.TypeVar("KindT", bound=VirtualKind)
SpecT = typing.TypeVar("SpecT")
SpecParams = typing.ParamSpec("SpecParams")


class MzBuilderMixin(typing.Generic[SpecT]):
    """Provide a more python interface around builders with specs.

    This allows for builders to be used as context managers (exposes their spec).
    """

    _spec: SpecT

    def __enter__(self) -> typing.Self:
        return self

    def __exit__(self, exc_type, exc_value, traceback):
        _ = exc_type, exc_value, traceback

    @contextlib.contextmanager
    def spec_context(self):
        """Support entering the underlying spec."""
        yield self._spec


class MzWrapper(typing.Generic[KindT, SpecT]):
    """Wrapper around a Grafana Foundation SDK builder, to provide a more pythonic interface.

    This is internal as heck and ugly.
    End users shouldn't have to interact with this.
    Sorry for anyone who does. ^^;
    """

    def __init__(
        self,
        kind_cls: type[KindT],
        spec_cls: Callable[SpecParams, SpecT],
        *args: SpecParams.args,
        **kwargs: SpecParams.kwargs,
    ):
        """Initialize the wrapper."""
        self.kind_cls = kind_cls
        # normally we'd use a type[] here, but we want to capture the ParamSpec
        assert isinstance(spec_cls, type)
        # We do a lot of "magic" to get really concrete matching types here
        # (basically, *args and **kwargs are validated against spec_cls)
        self.spec_cls = spec_cls
        self.args = args
        self.kwargs = kwargs
        self.item: KindT | None = None

    def build(self) -> KindT:
        """Build the wrapped object."""
        if not self.item:
            self.item = self.kind_cls(spec=self.spec_cls(*self.args, **self.kwargs))
        return self.item

    def __enter__(self) -> SpecT:
        """Enter the context manager."""
        spec = self.build().spec
        assert isinstance(self.spec_cls, type)
        assert isinstance(spec, self.spec_cls)
        return spec

    def __exit__(self, exc_type, exc_value, traceback):
        """Exit the context manager."""
        _ = exc_type, exc_value, traceback
