"""Layout builder.

The Grafana Foundation SDK provides a few builders for layouts, but they
are mostly passing around references instead of concrete panels, which
is not ideal for declarative maintainability.
"""

from __future__ import annotations

import contextlib
import typing
import weakref
from collections.abc import Iterator

from grafana_foundation_sdk.cog import builder as cogbuilder

from .builders_v2 import dashboardv2 as dashboardv2_builders
from .models_v2 import dashboardv2
from .wrapper import MzBuilderMixin, MzWrapper

type LayoutKind = (
    dashboardv2.GridLayoutKind
    | dashboardv2.AutoGridLayoutKind
    | dashboardv2.RowsLayoutKind
    | dashboardv2.TabsLayoutKind
)
type LayoutSpec = (
    dashboardv2.GridLayoutSpec
    | dashboardv2.AutoGridLayoutSpec
    | dashboardv2.RowsLayoutSpec
    | dashboardv2.TabsLayoutSpec
)
type LayoutItemKind = (
    dashboardv2.GridLayoutItemKind
    | dashboardv2.AutoGridLayoutItemKind
    | dashboardv2.RowsLayoutRowKind
    | dashboardv2.TabsLayoutTabKind
)
type LayoutItemSpec = (
    dashboardv2.GridLayoutItemSpec
    | dashboardv2.AutoGridLayoutItemSpec
    | dashboardv2.RowsLayoutRowSpec
    | dashboardv2.TabsLayoutTabSpec
)
LayoutKindT = typing.TypeVar("LayoutKindT", bound=LayoutKind)
LayoutSpecT = typing.TypeVar("LayoutSpecT", bound=LayoutSpec)
LayoutItemKindT = typing.TypeVar("LayoutItemKindT", bound=LayoutItemKind)
LayoutItemSpecT = typing.TypeVar("LayoutItemSpecT", bound=LayoutItemSpec)


class LayoutElements(typing.NamedTuple):
    """Built layout and its elements."""

    layout: LayoutKind
    elements: dict[str, dashboardv2.Element]
    """Elements are panels (PanelKind or LibraryPanelKind)."""


class MzLayoutBuilder(MzWrapper[LayoutKindT, LayoutSpecT]):
    """Builder class for building dashboard layouts.

    Instead of providing a series of builders explicitly, this exposes
    the builder objects you'd want using the context manager protocol
    (python `with` statements).
    """

    def __init__(self, *args, **kwargs):
        """Initialize the layout builder."""
        self._parent: weakref.ref[MzLayoutBuilder] | None = None
        self.panels: dict[str, dashboardv2.Element] = {}
        self.layout_items: list[MzLayoutBuilder] = []
        super().__init__(*args, **kwargs)

    @property
    def parent(self) -> MzLayoutBuilder | None:
        """Get the parent layout builder."""
        if self._parent is None:
            return None
        value = self._parent()
        assert value is not None, "lost reference to parent"
        assert isinstance(value, MzLayoutBuilder)
        return value

    @parent.setter
    def parent(self, value: MzLayoutBuilder | None):
        """Set the parent layout builder."""
        if value is None:
            self._parent = None
        else:
            self._parent = weakref.ref(value)

    def add_tab(self, tab_name: str, **kwargs) -> MzLayoutBuilder:
        """Add a tab to this layout."""
        assert self.layout_type in (None, dashboardv2.TabsLayoutKind), (
            "Can only add tabs to a TabsLayoutKind"
        )
        self.layout_type = dashboardv2.TabsLayoutKind
        child = MzLayoutBuilder(parent=self)
        self.layout_items.append(child)
        return child

    def add_row(self, **kwargs) -> MzLayoutBuilder:
        """Add a row to this layout."""
        assert self.layout_type in (None, dashboardv2.RowsLayoutKind), (
            "Can only add rows to a RowsLayoutKind"
        )
        self.layout_type = dashboardv2.RowsLayoutKind
        child = MzLayoutBuilder(parent=self)
        self.layout_items.append(child)
        return child


class MzLayoutItemBuilder(typing.Generic[LayoutItemSpecT]):
    """Builder class for sub-layout items."""

    def __init__(self, parent: MzLayoutBuilder):
        """Initialize the layout item builder."""
        self.parent = parent


@typing.runtime_checkable
class _LayoutBuilderProtocol(typing.Protocol):
    def layout(self, item: ...) -> None: ...


class MzLayoutMixin:
    """This allows for components that support setting layouts to expose a builder."""

    _has_layout = False

    @contextlib.contextmanager
    def add_tab_layout(self, **kwargs) -> Iterator[MzTabsLayoutBuilder]:
        """Set a tab layout.

        This must be used as a context manager.
        """
        assert isinstance(self, _LayoutBuilderProtocol)
        assert not self._has_layout
        self._has_layout = True
        layout_builder = MzTabsLayoutBuilder(**kwargs)
        yield layout_builder
        self.layout(layout_builder)

    @contextlib.contextmanager
    def add_row_layout(self, **kwargs) -> Iterator[MzRowsLayoutBuilder]:
        """Set a row layout.

        This must be used as a context manager.
        """
        assert isinstance(self, _LayoutBuilderProtocol)
        assert not self._has_layout
        self._has_layout = True
        layout_builder = MzRowsLayoutBuilder(**kwargs)
        yield layout_builder
        self.layout(layout_builder)

    @contextlib.contextmanager
    def add_auto_grid_layout(self, **kwargs) -> Iterator[MzAutoGridLayoutBuilder]:
        """Set an auto grid layout.

        This must be used as a context manager.
        """
        assert isinstance(self, _LayoutBuilderProtocol)
        assert not self._has_layout
        self._has_layout = True
        layout_builder = MzAutoGridLayoutBuilder(**kwargs)
        yield layout_builder
        self.layout(layout_builder)

    @contextlib.contextmanager
    def add_grid_layout(self, **kwargs) -> Iterator[MzGridLayoutBuilder]:
        """Set a grid layout.

        This must be used as a context manager.
        """
        assert isinstance(self, _LayoutBuilderProtocol)
        assert not self._has_layout
        self._has_layout = True
        layout_builder = MzGridLayoutBuilder(**kwargs)
        yield layout_builder
        self.layout(layout_builder)


class MzTabBuilder(
    MzLayoutMixin,
    MzBuilderMixin[dashboardv2.TabsLayoutTabSpec],
    dashboardv2_builders.Tab,
):
    """Builder for a single tab."""

    def __init__(self, **kwargs):
        self._spec = dashboardv2.TabsLayoutTabSpec(**kwargs)
        self._internal = dashboardv2.TabsLayoutTabKind(spec=self._spec)

    def add_tab_layout(self, **kwargs):
        _ = kwargs
        raise RuntimeError("Cannot add a tab layout to a tab builder")


class MzTabsLayoutBuilder(
    MzBuilderMixin[dashboardv2.TabsLayoutSpec], dashboardv2_builders.Tabs
):
    """Wrapper around the Tabs builder."""

    def __init__(self, **kwargs):
        self._spec = dashboardv2.TabsLayoutSpec(**kwargs)
        self._internal = dashboardv2.TabsLayoutKind(spec=self._spec)
        self._builders: list[cogbuilder.Builder[dashboardv2.TabsLayoutTabKind]] = []

    def add_tab(self, title: str, **kwargs) -> MzTabBuilder:
        """Add a single tab to this tab layout.

        Prefer to use this as a context manager.
        """
        tab_builder = MzTabBuilder(title=title, **kwargs)
        self._builders.append(tab_builder)
        return tab_builder

    def build(self) -> dashboardv2.TabsLayoutKind:
        self.tabs(self._builders)
        return super().build()


class MzGridItemBuilder(
    MzLayoutMixin,
    MzBuilderMixin[dashboardv2.GridLayoutItemSpec],
    dashboardv2_builders.GridItem,
):
    """Wrapper around a single grid item."""

    def __init__(self, **kwargs):
        self._spec = dashboardv2.GridLayoutItemSpec(**kwargs)
        self._internal = dashboardv2.GridLayoutItemKind(spec=self._spec)


class MzGridLayoutBuilder(
    MzBuilderMixin[dashboardv2.GridLayoutSpec], dashboardv2_builders.Grid
):
    """Wrapper around the Grid builder."""

    def __init__(self, **kwargs):
        self._spec = dashboardv2.GridLayoutSpec(**kwargs)
        self._internal = dashboardv2.GridLayoutKind(spec=self._spec)

    def add_grid_item(self, **kwargs) -> MzGridItemBuilder:
        """Add a grid item to this grid layout.

        Prefer to use this as a context manager.
        """
        item_builder = MzGridItemBuilder(**kwargs)
        return item_builder


class MzAutoGridItemBuilder(
    MzLayoutMixin,
    MzBuilderMixin[dashboardv2.AutoGridLayoutItemSpec],
    dashboardv2_builders.AutoGridItem,
):
    """Wrapper around a single auto grid item."""

    def __init__(self, **kwargs):
        self._spec = dashboardv2.AutoGridLayoutItemSpec(**kwargs)
        self._internal = dashboardv2.AutoGridLayoutItemKind(spec=self._spec)


class MzAutoGridLayoutBuilder(
    MzBuilderMixin[dashboardv2.AutoGridLayoutSpec], dashboardv2_builders.AutoGrid
):
    """Wrapper around the AutoGrid builder."""

    def __init__(self, **kwargs):
        self._spec = dashboardv2.AutoGridLayoutSpec(**kwargs)
        self._internal = dashboardv2.AutoGridLayoutKind(spec=self._spec)


class MzRowBuilder(
    MzLayoutMixin,
    MzBuilderMixin[dashboardv2.RowsLayoutRowSpec],
    dashboardv2_builders.Row,
):
    """Wrapper around a single row."""

    def __init__(self, **kwargs):
        self._spec = dashboardv2.RowsLayoutRowSpec(**kwargs)
        self._internal = dashboardv2.RowsLayoutRowKind(spec=self._spec)


class MzRowsLayoutBuilder(
    MzBuilderMixin[dashboardv2.RowsLayoutSpec], dashboardv2_builders.Rows
):
    """Wrapper around the Rows builder."""

    def __init__(self, **kwargs):
        self._spec = dashboardv2.RowsLayoutSpec(**kwargs)
        self._internal = dashboardv2.RowsLayoutKind(spec=self._spec)

    def add_row(self, **kwargs) -> MzRowBuilder:
        """Add a row to this row layout.

        Prefer to use this as a context manager.
        """
        row_builder = MzRowBuilder(**kwargs)
        return row_builder
