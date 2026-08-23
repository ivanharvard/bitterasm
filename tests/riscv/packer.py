"""A generic "walk an EmittedValue's fields and concatenate their bits"
packer — a Python stand-in for the job `bitter`'s own encoder is meant to
do (see the design discussion in the project history: `bitter` never needs
to know it's looking at RISC-V, just that a field is `bits<N>`-shaped).

Deliberately architecture-blind: it has no idea what RType/IType/opcode/
funct3 mean, it just concatenates whatever `bits<N>` fields a struct
carries, in the order bitterasm's `compile` emitted them.
"""

from __future__ import annotations

import json


def _field_bits(value: dict) -> str:
    if value["kind"] == "Struct" and value["name"] == "bits":
        width = int(value["args"][0]["value"])
        inner = dict(value["fields"])["value"]
        bit_value = int(inner["value"])
        return format(bit_value & ((1 << width) - 1), f"0{width}b")

    raise ValueError(f"not a bits<N>-shaped value: {value!r}")


def pack(value: dict) -> int:
    if value["kind"] != "Struct":
        raise ValueError(f"expected a struct-shaped emitted value, found {value!r}")

    bitstring = "".join(_field_bits(field) for _name, field in value["fields"])
    return int(bitstring, 2)


def pack_file(path: str) -> list[int]:
    with open(path) as handle:
        values = json.load(handle)

    return [pack(value) for value in values]
