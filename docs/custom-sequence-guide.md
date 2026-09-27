# Custom sequence guide

By default, `repro-toolkit` builds the UDS repro sequence entirely by
parsing the PDX file. If you need to change that sequence — insert a
vendor-specific step, add a delay, reorder steps, or replace the whole
thing — supply a **custom sequence JSON file** alongside the PDX. When
you don't supply one, nothing changes: the library falls back to its
default, PDX-derived sequence.

- CLI: `repro-toolkit parse ecu.pdx --sequence my-sequence.json`
- Library (Rust): `repro_toolkit_core::generate_sequence(pdx_path, Some(sequence_path))`
- FFI (C#/Python/C++): `repro_toolkit_generate_sequence(pdx_path, sequence_path)` — pass `NULL`/`nullptr` for `sequence_path` to get the default behavior.

A worked example lives at [`samples/custom-sequence.example.json`](../samples/custom-sequence.example.json).

## Why a full sequence instead of a diff/overlay

You author the *entire* sequence, not a patch on top of the default one.
This keeps the format simple (it's plain JSON, with no "insert after
step X" indirection to get wrong) and means the exact JSON this library
*outputs* is also valid *input* — you can generate the default sequence,
hand-edit the result, and feed it back in as your custom sequence.

## Top-level shape

```json
{
  "ecu_variant": "MyECU_Variant",
  "steps": [ /* ... */ ]
}
```

| Field         | Required? | Notes                                                                 |
|---------------|-----------|------------------------------------------------------------------------|
| `ecu_variant` | No        | If omitted, the ECU variant name is read from the PDX passed alongside this file. |
| `steps`       | Yes       | Must contain at least one step.                                       |

You never set `index` or `step_count` yourself — both are computed
automatically from `steps`.

## Step shape

```json
{
  "name": "VendorPreFlashSelfTest",
  "category": "OTHER",
  "semantic": "FLASH",
  "notes": "Optional free-text note for humans or the flashing tool.",
  "request": { "name": "...", "fields": [ /* ... */ ] },
  "expected_positive_responses": [ { "name": "...", "fields": [ /* ... */ ] } ],
  "expected_negative_responses": [ { "name": "...", "fields": [ /* ... */ ] } ]
}
```

| Field                          | Required? | Default    |
|---------------------------------|-----------|------------|
| `name`                          | Yes       | —          |
| `category`                      | No        | `"OTHER"`  |
| `semantic`                      | No        | `null`     |
| `request`                       | No        | `null`     |
| `expected_positive_responses`   | No        | `[]`       |
| `expected_negative_responses`   | No        | `[]`       |
| `notes`                         | No        | `null`     |

A step with no `request` at all (just a `name` and `notes`) is valid —
useful for a step that isn't a UDS request, like a required delay, that
you still want a flashing tool to see and act on in order.

### `category`

One of the following. It's informational — tools built on this library
can use it to recognize a step's role (e.g. to know which step supplies
the security key, or which steps to repeat per flash block); it never
changes step ordering — **the order of `steps` in your JSON is the order
the tool will use**, exactly as written.

`SESSION_CONTROL`, `SECURITY_SEED_REQUEST`, `SECURITY_KEY_SEND`,
`ERASE_MEMORY`, `REQUEST_DOWNLOAD`, `TRANSFER_DATA`,
`REQUEST_TRANSFER_EXIT`, `CHECK_MEMORY`, `ECU_RESET`, `OTHER`.

Use `OTHER` (or omit `category` entirely) for anything without a
built-in meaning here: a vendor-specific routine, a settle delay, an
extra verification step.

### `request` / response messages

Each of `request` and the entries in `expected_positive_responses` /
`expected_negative_responses` has the same shape:

```json
{ "name": "TransferData_Req", "fields": [ /* field objects, see below */ ] }
```

### Fields

Each field is either **fixed** (a byte value you already know — a
service ID, a sub-function, an OEM-defined constant) or **variable** (a
value only known at the time the sequence actually runs — a security
seed, an address, a data payload):

```json
{ "name": "SID", "byte_position": 0, "bit_length": 8, "kind": "fixed", "value_hex": "10" }
{ "name": "Seed", "byte_position": 2, "kind": "variable", "data_type": "A_BYTEFIELD" }
```

| Field           | Applies to        | Required? | Notes                                              |
|-----------------|-------------------|-----------|------------------------------------------------------|
| `name`          | both              | Yes       |                                                       |
| `byte_position` | both              | No        | Byte offset in the message, if known/relevant.       |
| `bit_length`    | both              | No        | Bit width, if known/relevant.                        |
| `kind`          | both              | Yes       | `"fixed"` or `"variable"`.                           |
| `value_hex`     | `kind: "fixed"`   | Yes       | Uppercase or lowercase hex, no `0x` prefix, e.g. `"10"`, `"31"`. |
| `data_type`     | `kind: "variable"`| No        | ODX base data type if known (`A_UINT32`, `A_BYTEFIELD`, ...) — purely informational. |

## Common recipes

**Insert a custom step between two standard ones** — just place it in
`steps` where you want it to run; there's no anchor/reference syntax
needed, since you're writing the whole ordered list:

```json
{ "name": "StandardStepBefore", "...": "..." },
{ "name": "MyVendorStep", "category": "OTHER", "request": { "...": "..." } },
{ "name": "StandardStepAfter", "...": "..." }
```

**Add a delay with no request/response at all:**

```json
{ "name": "SettleDelay", "notes": "wait 500ms before requesting security access" }
```

**Start from the default sequence and edit it:** generate the default
JSON first, then hand-edit it and pass it back as your custom sequence:

```
repro-toolkit parse ecu.pdx -o default-sequence.json
# edit default-sequence.json ...
repro-toolkit parse ecu.pdx --sequence default-sequence.json
```

## Validation

The loader rejects:
- a file with zero steps,
- an unrecognized `category` value (must be one of the values listed above, or omitted),
- any field that doesn't match the shapes above (e.g. `kind: "fixed"` missing `value_hex`).

Errors name the file path and the underlying JSON error to make fixing a
malformed sequence file straightforward.
