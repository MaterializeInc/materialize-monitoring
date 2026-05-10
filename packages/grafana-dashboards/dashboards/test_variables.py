"""Unit tests for common variables.

These tests are to be focused on BEST PRACTICES
and not every possible implementation detail.
"""

from __future__ import annotations

import typing

import pytest

from . import variables

VARIABLE_NAMES = [
    value
    for key, value in vars(variables.VariableNames).items()
    if not key.startswith("_")
]


def test_variable_names_all_strings():
    """Test that all variable names are strings."""
    hints = typing.get_type_hints(variables.VariableNames)
    for key, value in vars(variables.VariableNames).items():
        if key.startswith("_"):
            # ignore python dunder magic attributes (and privates)
            continue
        assert key in hints, f"Variable name {key} is missing a type hint"
        assert isinstance(value, str), f"Variable name {key} is not a string"

        assert hints[key] == typing.Final[str], (
            f"Variable name {key} is not typed as Final[str]"
        )


@pytest.mark.parametrize("variable_name", VARIABLE_NAMES)
def test_variable_naming(variable_name: str):
    """Test that all variable names are camelCase."""
    assert 1 <= len(variable_name) <= 20, "variable names should be reasonably sized"
    assert not variable_name[0].isupper()
    assert "_" not in variable_name, "do not use snake case"


def test_environment_variable():
    """Test that the environment variable is configured appropriately."""
    var = variables.environment_variable().build()
    assert "environment" in var.spec.name, var.spec.name
    assert not var.spec.name.endswith("s"), "must be singular"
    assert var.spec.allow_custom_value is True, "must allow break glass values"
    assert var.spec.multi is False, "single environment is limited to one value"


def test_metrics_datasource_variable():
    """Test that the metrics datasource is configured appropriately."""
    var = variables.metrics_datasource().build()
    assert var.spec.name == variables.VariableNames.METRIC_DS
    assert "prometheus" in var.spec.plugin_id, var.spec.plugin_id
    assert var.spec.allow_custom_value is False, (
        "datasource should not allow custom values"
    )
