# x86-64 implementation progress

**Read this file before touching anything in `std/x86_64/` or the associated
`tests/x86_64*` files.** It is the source of truth for what's done, what's next, and
which design decisions are already settled — not any prior chat conversation.

## Status

- [x] Phase 0 — Scaffold + progress tracker
- [x] Phase 1 — Encoding primitives
- [x] Phase 2 — Register-direct instructions: mov, ALU ops, test
- [x] Phase 3 — Memory operands: ModRM/SIB addressing engine
- [x] Phase 4 — Control flow: jmp/jcc/call/ret
- [x] Phase 5 — Remaining core subset: shl/shr/sar, lea, push/pop
- [x] Phase 6 — Native (Intel-syntax) dialect
- [ ] Phase 7 — Independent-oracle cross-check (GNU binutils)

Work through phases in order, one at a time. Each phase's section below has enough
context to pick up cold — deliverable, files, open questions, and how to verify. When
you finish a phase, check it off above and fill in any "resolved as" notes the phase
left open, before ending your session.

## Decided scope (do not revisit without a real reason)

Core GPR-only integer subset: `mov`, `add`/`sub`/`and`/`or`/`xor`/`cmp`/`test`,
`shl`/`shr`/`sar`, `lea`, `push`/`pop`, `jmp`/`jcc` (all condition codes)/`call`/`ret`.
32-bit and 64-bit operand sizes only — no 8/16-bit registers, no `0x66` operand-size
prefix, no SSE/AVX/string/BCD instructions. Memory operands cover `[base]`,
`[base+disp8/32]`, `[base+index*scale+disp]`, and RIP-relative. Branches are near-only
(`rel32`); no automatic short/near relaxation — that's an explicit macro/dialect
choice if it's ever added, not automatic assembler behavior (this repo's own stated
design philosophy: code-selection belongs to the macro author, not the language —
see README.md's "Abstraction does not imply optimization").

**Explicitly out of scope for v1** (don't add without discussing first — these were
deliberately deferred, not overlooked): 8-bit/16-bit registers, `0x66` operand-size
prefix, AT&T-syntax dialect, explicit short (`rel8`) jump/call variants, SSE/AVX,
string/BCD instructions, far pointers.

## Established convention (already followed by `std/riscv/`, `std/pdp10/`)

- `std/x86_64/impl.basm` — base macros (positional args), registers, field-width type
  aliases, instruction-format structs.
- `std/x86_64/<dialect>.basm` — pure re-syntaxing via `syntax name(args) = { pattern
  }`, no new semantics.
- `tests/x86_64_encoding.rs` / `tests/x86_64_dialects.rs` — real CLI-driven
  (`bitterasm compile` + `bitter encode` as actual subprocesses) byte-exact tests.
- `tests/fixtures/x86_64/` — small `.basm` fixtures those tests reference.
- `tests/x86_64/` (Phase 7) — independent-oracle cross-check harness (Docker + GNU
  binutils), mirroring `tests/riscv/`.

## Key language/packer facts (verified against source, don't re-derive)

- `@if`/`@else`/`@match` are real compile-time control flow
  (`src/resolver/macro_body.rs`), not textual substitution — `src/expander.rs` is an
  unrelated `bitterasm expand` tool.
- Macros support real recursion (`std/wasm/leb128.basm:49-56`) and `@for i in 0..N`
  for generated fields/consts (`std/riscv/impl.basm:13` generates `x0..x31`).
- `bitter` (`bitter/src/pack.rs`) cannot pack a bare `Int` or `Enum` directly — only
  concrete `bits<N>`/`Positioned<N>` leaves (it special-cases the `Deferred` enum by
  name for label arithmetic). An `enum` can still be used freely *inside* macro logic
  (`@match` to pick a code path) as long as the final `@emit`/`@return` is a concrete
  struct. **This is how "optional fields" are modeled here: branch to a different
  concrete shape at compile time, never emit a sometimes-absent field.**
- The reusable pattern for x86's variable length: compute the needed byte count as an
  ordinary recursive compile-time `int` (like `std/wasm/leb128.basm`'s
  `uleb128_length`), then build one fully concrete `Bytes<N>` struct sized to that
  computed length (like `uleb128_padded`).
- `LittleEndian<T, const width: int>` (`std/bitter/byte_order.basm`) is recognized by
  name in the packer to byte-reverse a value once its total width is known.
- `Positioned<N>` (`std/bitter/deferred.basm`) carries a still-unresolved
  label-relative value plus its intended width, resolved by `bitter` once `here()` is
  known at pack time — basis for RISC-V's `beq`/`jal` and for x86's Phase 4
  `jmp`/`jcc`/`call rel32`.

## Phases

### Phase 0 — Scaffold + progress tracker ✅
Created this file and `std/x86_64/impl.basm` (skeleton: header + imports only) and
`tests/fixtures/x86_64/`.

### Phase 1 — Encoding primitives (no instructions yet) ✅
**Resolved:** `Reg` ended up as `pub type Reg = int` (plain alias, like
`std/unsigned.basm`'s `uint`), not `bits<4>` as originally sketched. Reason: a
`bits<N>` struct's `value` field is declared without `pub` in `std/binary.basm`, so
it can't be read back out from any other module (`PrivateFieldAccess`) — and
extracting a register's low 3 bits / extension bit is exactly what `reg_field`/
`reg_ext` need to do. `rex_byte`/`modrm_byte`/`sib_byte` take plain `int` arguments
for the same reason, doing all bit arithmetic (`<<`/`|`/`&`) in `int` and wrapping
the result in `Byte(...)` only once, at the very end — the same idiom RISC-V's
`slli`/`srai` already use for their own shift-immediate encoding.
**Gotcha for future verification-by-`@assert` scratch files:** `bitterasm check`
does *not* evaluate a macro's body — including its `@assert`s — unless that macro is
actually invoked somewhere in the file (top-level `name` or `name()`, no `@emit`
required if there's nothing to emit). An unused `macro check() { @assert ... }` with
no call silently "passes" without checking anything; confirmed by deliberately
breaking an assertion and seeing `bitterasm check` still report success until the
call was added. Always end a scratch fixture with a bare invocation of its
test/check macro, and once, verify a deliberately-wrong assertion actually fails
before trusting the rest.

<!-- original phase description below, kept for reference -->

**Deliverable:** the byte-building toolkit every instruction macro will call into.
- `pub type Reg = bits<4>` + 16 named registers `r0..r15` via `@for` (like RISC-V's
  `x0..x31`) + ABI aliases `rax=r0, rcx=r1, rdx=r2, rbx=r3, rsp=r4, rbp=r5, rsi=r6,
  rdi=r7` (r8-r15 keep their numeric names, matching real x86-64 convention).
- `pub type Byte = bits<8>` and `pub struct Bytes<const N: int>` — own copy here,
  deliberately not shared with `std/wasm/leb128.basm`'s identical-looking type.
  RISC-V's own comments (`std/riscv/impl.basm:55-61`) already embrace this kind of
  per-architecture duplication for self-documentation — follow that precedent rather
  than introducing a cross-domain `std/bitter/bytes.basm` dependency.
- `reg_field(r: Reg) -> bits<3>` (low 3 bits) and `reg_ext(r: Reg) -> bits<1>` (bit 3).
- `rex_byte(w: bits<1>, r: bits<1>, x: bits<1>, b: bits<1>) -> Byte` (`0100WRXB`).
- `modrm_byte(mod: bits<2>, reg: bits<3>, rm: bits<3>) -> Byte`.
- `sib_byte(scale: bits<2>, index: bits<3>, base: bits<3>) -> Byte`.
- `byte_of(value: int, index: int) -> Byte` — little-endian byte `index` of `value`
  (`(value >> (8*index)) & 0xFF`), reusable for any width immediate/displacement.
**Open questions:** none — straightforward arithmetic/struct code.
**Files:** `std/x86_64/impl.basm` (extend).
**Verification:** no `cargo test` harness yet — add a handful of `@assert`s under a
temporary scratch macro (or a throwaway fixture under `tests/fixtures/x86_64/`)
checking known byte values, e.g. `rex_byte(1,0,0,1) == Byte(0b01001001)`,
`modrm_byte(0b11, 0b000, 0b001) == Byte(0xC1)`; run via `bitterasm check`. Delete the
scratch file once Phase 2 has real instructions exercising the same code paths.

### Phase 2 — Register-direct instructions: mov, ALU ops, test ✅
**Resolved:**
- **Direction convention:** every reg,reg opcode is the "r/m, r" form (`0x89`-style
  — ModRM.rm is the destination, ModRM.reg is the source). With `mod=11`
  register-direct addressing this is purely a byte-encoding choice, not a semantic
  one (`0x8B`, "r, r/m", would decode identically) — `0x89` was picked so `mov`'s own
  opcode byte matches its mnemonic's conventional Intel-manual table entry. Every
  macro keeps `(rd, rs, ...)` argument order, destination first, so Phase 6's
  Intel-syntax dialect can read `mov rd, rs` directly off the positional order.
- **Operand size:** an explicit trailing `w: int` parameter (0 = 32-bit, 1 = 64-bit,
  forcing REX.W) on every instruction macro, exactly as this phase's own original
  plan below already specified ("an `@if` branch on `w`") — not two macro families,
  not overloading (see next point for why overloading doesn't apply here anyway). A
  REX byte is only actually emitted when `w`, or an extended (r8-r15) register,
  actually needs one — confirmed byte-exact both ways in `regdirect.basm`'s `mov r8,
  r9, 0` (REX for extended regs, `w=0`) vs `mov rax, rbx, 1` (REX for `w=1` only)
  cases.
- **reg,imm forms use separate names, not macro overloading:** `movi`/`addi`/`ori`/
  `andi`/`subi`/`xori`/`cmpi`/`testi`, following `std/riscv/impl.basm`'s own
  established `i`-suffix convention (`addi`/`andi`/...), rather than overloading
  `mov`/`add`/... by parameter type. Macro overloading is a real, already-implemented
  language feature (dispatches on resolved argument type or arity — see
  `src/resolver/macro_body.rs`'s `resolve_macro_overload` and its
  `macro_overloads_dispatch_by_resolved_argument_type` test) but it can't distinguish
  these two forms here: `Reg` is a plain `int` alias (see Phase 1's own resolution
  above), so `mov(rd: Reg, rs: Reg, w: int)` and `mov(rd: Reg, imm: int, w: int)`
  both resolve to identical `(int, int, int)` parameter types and would be rejected
  as ambiguous (confirmed against that same test file's
  `identical_macro_overloads_are_ambiguous_at_the_call_site`). Overloading only
  becomes usable once two forms resolve to genuinely distinct types — exactly what
  Phase 3's `MemOperand` will be for reg,mem vs. reg,reg (real macro overloading is
  still on the table for that phase, per its own deliverable below).
- **Shared internal helpers:** `reg_reg_instr(opcode, rd, rs, w)` (mov, the six ALU
  ops, and `test` all share this — one more instruction than the phase's original
  plan asked to factor, since `mov`/`test` turned out to have the exact same
  rex?+opcode+modrm shape as the ALU ops) and `reg_imm_instr(opcode, digit, rd, imm,
  w)` (the six ALU reg,imm forms and `testi`, which only differ in opcode byte:
  `0x81` vs `0xF7`). Both bottom out in one general `instr(need_rex, rex, opcode,
  has_modrm, modrm, imm, imm_len)` that builds a single `Bytes<N>` via the
  `uleb128_padded`-style "compute length, then `@for i in 0..n`" idiom this file's
  own header already pointed at, with a per-index `instr_byte` helper picking
  REX-vs-opcode-vs-ModRM-vs-immediate the same guard-chain way
  `std/string.basm`'s UTF-8 decoding classifies a byte.
- **`@emit` vs `@return`:** every public instruction macro's body is `@emit
  reg_reg_instr(...)` (or `reg_imm_instr`/`instr`), not `@return` — `@return` alone
  produces nothing when the macro is invoked as a bare top-level statement (confirmed
  empirically: an early draft using `@return` throughout compiled clean but emitted 0
  bytes). `-> Bytes<...>` return-type annotations stay on these macros anyway, purely
  as documentation — `std/wasm/impl.basm`'s `unreachable`/`br`/etc. already do the
  same (declare a return type, `@emit` in the body). The internal helpers above
  (`reg_reg_instr`, `reg_imm_instr`, `instr`, `instr_byte`) are ordinary `@return`-based
  value-computing functions, called only from other macros' expression position, never
  invoked as bare statements.
**Files:** `std/x86_64/impl.basm` (extended); `tests/fixtures/x86_64/regdirect.basm`
(new, 21 cases covering every mnemonic, the REX/no-REX split, and both operand
sizes); `tests/x86_64_encoding.rs` (new, byte-exact against one hand-computed
expected vector for the whole fixture — no independent oracle exists yet at this
phase, matching `tests/pdp10_encoding.rs`'s own standard). Phase 1's
`tests/fixtures/x86_64/scratch_primitives.basm` scratch fixture is deleted, per its
own note, now that `regdirect.basm` exercises the same primitives for real.
**Verification:** `cargo test --test x86_64_encoding` passes (21/21 emitted values,
89 bytes, matching hand-computed expected output exactly).

<!-- original phase description below, kept for reference -->

**Deliverable:** first real, byte-verified instructions, register-direct addressing
only (ModRM `mod=11`), both operand sizes, extended registers r8-r15.
- `mov` (reg,reg via `0x89`; reg,imm32 via `0xB8+reg` with an imm32 or imm64 depending
  on REX.W — same opcode family needs a width-dependent immediate size, an `@if`
  branch on `w`).
- `add/sub/and/or/xor/cmp` reg,reg (opcode+ModRM, mod=11) and reg,imm32 (`0x81
  /digit`, ModRM.reg holds a per-mnemonic 3-bit opcode extension instead of a real
  register). Implement these six as thin wrappers around one shared internal helper
  (e.g. `alu_reg_reg(opcode_base, rd, rs)` / `alu_reg_imm(digit, rd, imm)`) rather
  than copy-pasting per-mnemonic — x86's ALU family genuinely shares one encoding
  shape differing only by a 3-bit constant, unlike RISC-V's `funct3`-only
  differences; the shared structure here is real, so factor it.
- `test` reg,reg (`0x85`) and reg,imm32 (`0xF7 /0`).
**Open question to settle here:** pick and document one canonical opcode/direction
convention per instruction (e.g. `0x89` vs `0x8B` for `mov` reg,reg both encode the
same op with reg/rm swapped) — impl.basm's positional argument order should match
whichever direction is chosen, the same way RISC-V's impl.basm picks one canonical
order that dialects later re-syntax, not re-derive. Record the choice here once made.
**Resolved as:** see "Resolved" block above — `0x89`-style "r/m, r" direction,
`(rd, rs, ...)` destination-first argument order.
**Files:** `std/x86_64/impl.basm` (extend); `tests/fixtures/x86_64/regdirect.basm`
(new); `tests/x86_64_encoding.rs` (new, byte-exact, hand-computed expected bytes from
the Intel SDM — same convention as `tests/pdp10_encoding.rs` since no oracle exists
yet at this phase).
**Verification:** `cargo test --test x86_64_encoding`.

### Phase 3 — Memory operands: ModRM/SIB addressing engine ✅
**Resolved:**
- **disp8 vs disp32:** chosen automatically from the literal displacement's own
  magnitude (`fits_disp8`: `-128 <= disp <= 127`), exactly the recommendation this
  phase's own original plan below already made — pure encoding-size selection, no
  relaxation, no conflict with the no-relaxation decision.
- **`MemOperand` shape:** one `enum MemOperand { Base: MemBase, Indexed: MemSib,
  RipRelative: MemRip }`, each variant wrapping its own named field struct (an enum
  variant carries exactly one typed payload — `src/ast.rs`'s
  `EnumVariantDeclaration` — so a multi-field variant needs a struct the same way
  `std/bitter/deferred.basm`'s `Deferred.Node: BinOp` already does), built only
  through the three constructor macros `Mem`/`MemIndexed`/`MemRipRelative` the
  original plan asked for. Every reg/mem-accepting overload only ever needs to know
  about the one `MemOperand` type; `addr_mode` is the sole place that ever
  `@match`es on which variant a value actually is.
- **`MemIndexed` guards:** `@assert`s `scale` is 1/2/4/8 and that `index`'s low 3
  bits aren't `0b100` (SIB.index=100 always means "no index," so rsp/r12 can never
  be a real index register) — both confirmed to actually fire (not silently inert,
  per Phase 1's own `@assert`-evaluation gotcha) by deliberately triggering each
  against a throwaway `.basm` file and seeing `bitterasm check` reject it.
- **The two ModRM/SIB special cases, handled in `addr_mode`:** a base register with
  low3=`0b100` (rsp/r12) always routes through a SIB byte with `index=0b100`
  ("none"), since ModRM.rm=`100` can never mean a real base register. A base
  register with low3=`0b101` (rbp/r13) *and* a literal zero displacement is
  promoted from `mod=00` to `mod=01 disp8=0`, since `mod=00,rm=101` means
  RIP-relative (or, inside a SIB byte, `mod=00,SIB.base=101` means "no base,
  disp32") rather than "no displacement" the way every other register's `mod=00`
  does — a non-zero displacement never hits this ambiguity on its own account
  (`mod` is already 01/10), so the forcing only applies in the `disp==0` case. Both
  confirmed byte-exact in `regmem.basm`'s `Mem(rsp, 0)`/`Mem(r12, 0)` (SIB forcing)
  and `Mem(rbp, 0)`/`Mem(r13, 0)`/`MemIndexed(rbp, ...)` (disp8=0 forcing) cases.
- **No bare `[disp32]` absolute addressing:** deliberately not built — this file's
  own "Decided scope" section only commits to `[base]`, `[base+disp]`,
  `[base+index*scale+disp]`, and RIP-relative; a base-less, index-less absolute form
  would need a fourth constructor nothing asks for.
- **The overload, and why `mov`'s memory-immediate form needed a genuinely
  different opcode:** every reg/mem instruction from Phase 2 (`mov`, the six ALU
  ops, `test`, and their `i`-suffixed reg,imm siblings) gained a `MemOperand`
  overload of its first (destination) parameter, real macro overloading this time
  (unlike Phase 2's `Reg`-vs-`int` case) since `MemOperand` is a genuinely distinct
  resolved type from `Reg`/`int`. `movi`'s register form folds the destination into
  opcode `0xB8+reg`, but there's no register to fold in when the destination is
  memory — its `MemOperand` overload uses the completely different `0xC7 /0`
  opcode instead, landing on the same reg,imm shape every ALU `i`-form already
  uses.
- **Two new general assembly helpers** (`mem_instr`/`mem_instr_byte`) generalize
  Phase 2's `instr`/`instr_byte` with two more optional sections (SIB, displacement)
  inserted between ModRM and the immediate, same "compute length, build one
  `Bytes<N>` via `@for`" idiom. Register-direct instructions are untouched and still
  go through Phase 2's original `instr` directly.
**Files:** `std/x86_64/impl.basm` (extended: `MemBase`/`MemSib`/`MemRip`/
`MemOperand`, `Mem`/`MemIndexed`/`MemRipRelative`, `scale_code`, `fits_disp8`,
`AddrMode`/`addr_mode`, `mem_instr`/`mem_instr_byte`, `mem_reg_instr`/
`mem_imm_instr`, and the `MemOperand` overloads of every Phase 2 instruction
macro); `tests/fixtures/x86_64/regmem.basm` (new, 16 cases: every `addr_mode`
branch — plain/disp8/disp32, both SIB-forcing registers, both disp8=0-forcing
registers, all three `MemIndexed` disp sizes plus its own forced-disp8=0 case, and
RIP-relative — plus 4 cases confirming `mem_reg_instr`/`mem_imm_instr` wiring for
`add`/`addi`/`movi`/`testi`); `tests/x86_64_encoding.rs` (extended with
`regmem_encodes_correctly`, same hand-computed-against-one-vector convention as
Phase 2's test).
**Verification:** `cargo test --test x86_64_encoding` passes (both
`regdirect_encodes_correctly` and `regmem_encodes_correctly`; 16/16 emitted values,
71 bytes, matching hand-computed expected output exactly).

<!-- original phase description below, kept for reference -->

**Deliverable:** a `MemOperand`-family of constructors and the addressing-mode
selection logic every reg/mem-accepting instruction needs — the hardest, most
open-ended phase.
- Separate named constructors rather than one giant optional-everything struct (e.g.
  `Mem(base: Reg, disp: int)`, `MemIndexed(base: Reg, index: Reg, scale: int, disp:
  int)`, `MemRipRelative(disp: int)`), mirroring how RISC-V's `native.basm` gives
  loads/stores their own `offset(rs1)` sugar rather than a generic addressing struct.
  Internally these can use an `enum` freely (`@match` to pick the right modrm/sib/disp
  shape) as long as each arm still bottoms out in a concrete `Bytes<N>` before
  `@emit`.
- mod selection: `mod=11` register-direct (Phase 2, done); `mod=00` no displacement
  (except `rm=101`, which is RIP-relative in 64-bit mode — the only "no base
  register" form v1 supports); `mod=01` disp8; `mod=10` disp32; `rm=100` forces a SIB
  byte; SIB `base=101` with `mod=00` means "no base, disp32 only".
- Overload `mov`/the ALU ops from Phase 2 to also accept a `MemOperand` in the rm
  position (macro overloading by parameter type).
**Open question to settle here:** disp8 vs disp32 chosen automatically from the
literal displacement's magnitude (mirroring `uleb128_length`'s magnitude-based group
count) — recommended, since this is pure encoding-size selection with no semantic
difference (unlike jump relaxation), so it doesn't conflict with the no-relaxation
decision.
**Resolved as:** see "Resolved" block above — automatic magnitude-based selection,
exactly as recommended.
**Files:** `std/x86_64/impl.basm` (extend); extend `tests/fixtures/x86_64/` and
`tests/x86_64_encoding.rs` with all four addressing forms.
**Verification:** `cargo test --test x86_64_encoding`.

### Phase 4 — Control flow: jmp/jcc/call/ret ✅
**Deliverable:** `jmp rel32` (`0xE9`), the ~16 `jcc rel32` forms (`0x0F 0x8_`, one
named macro per condition code — `je/jne/jl/jle/jg/jge/jb/jbe/ja/jae/js/jns/jo/jno/
jp/jnp`, matching real mnemonics, same style as RISC-V's separately-named
`beq/bne/blt/...`), `call rel32` (`0xE8`), `ret` (`0xC3`).
**Open question to settle BEFORE writing any code:** x86's `rel32` is relative to the
address of the *next* instruction (`target - (here() +
this_instruction's_own_byte_length)`), not relative to this instruction's own start
the way RISC-V's `here()`-based offset is. Read how `Positioned<N>`/`here()`
resolution actually works in `bitter/src/pack.rs` before implementing — if `here()`
only ever yields the current instruction's start address, the macro itself must add
its own (compile-time-known, since jmp/call/jcc are fixed-length) byte length to
correct for this. Do not guess. Record what you find and the resolution here.

**Investigated (prerequisite shared-infra change already made, instructions
themselves not yet written):**
- `here()`/`Deferred.Here` resolves to `here_index` — which top-level *entry* is
  currently being packed (0, 1, 2, ...) — never a byte address. RISC-V's
  `beq`/`bne`/... compute `mul(sub(target, here()), 4)`: entry-index delta × 4 =
  byte delta, correct *only* because every RV32I instruction is exactly 4 bytes
  (`std/riscv/impl.basm`'s own comment above `beq` says so explicitly). x86
  instructions are variable-length, so this exact trick doesn't generalize — no
  fixed multiplier converts an entry-index delta into the right byte delta.
- `bitter` *does* separately track every entry's real packed byte width
  (`byte_widths`, computed once up front in `pack_stream`) and had exactly one
  primitive built on it: `span(start: int, end: int)` (`std/bitter/deferred.basm`),
  used by `std/wasm/module.basm` for section-length prefixes. Two things blocked
  reusing it as-is for `rel32`: its public signature only accepted plain `int`
  endpoints (not `Deferred`, so `here()` couldn't be passed directly), and its
  resolution (`resolve_span` in `bitter/src/pack.rs`) hard-errored whenever
  `from > to` — deliberately, since its only caller always wants a non-negative
  length and treats a backward range as a bug. A relative branch needs both
  directions to work.
- **Resolved as:** generalized `span` itself rather than adding a parallel
  primitive. `resolve_span` (`bitter/src/pack.rs`) is now signed and bidirectional
  — `+Σbyte_widths[from..to]` if `from≤to`, else `-Σbyte_widths[to..from]` — with
  no error case for the reversed direction. `span` gained two overloads in
  `std/bitter/deferred.basm`: `(Deferred, int)` and `(int, Deferred)`, alongside
  the original `(int, int)` (not `(Deferred, Deferred)` — nothing needs it, and
  it's actually unsound to expose generally: two independently-captured `here()`
  values combined and read back from a *third* entry would both silently resolve
  to that third entry's own position instead of erroring, the same
  no-identity-of-its-own trap `deferred.basm`'s doc comment already warned about
  for the original signature — `span(here(), target)` stays sound only because the
  `here()` call and the `Positioned<N>` field it lands in are always the same
  entry). `jmp`/`jcc`/`call` will compute their `rel32` as `sub(span(here(),
  target), own_fixed_length)` — `own_fixed_length` is the compile-time-known
  correction for "relative to the *next* instruction" (5 for `jmp`/`call`, 6 for
  `jcc`) the open question above asked for, applied on top of a byte-accurate (not
  entry-index-accurate) span instead of on top of RISC-V's fixed multiplier.
  Verified via new Rust unit tests in `bitter/src/pack.rs` (a backward `span`
  returning a negative distance instead of erroring; a `here()` endpoint resolved
  bidirectionally against non-uniform per-entry byte widths — the exact shape a
  variable-length ISA's backward branch needs) and an extended
  `tests/fixtures/bitter/deferred_dispatch.basm` (`bitterasm check`-only,
  proving the new overloads dispatch to the right `Deferred` tree shape). Every
  existing `span`/`Deferred` consumer re-verified unaffected: `cargo test --lib`,
  `--test wasm_encoding`, `--test wasm_module` (including its independent-WASM-
  engine run), `--test riscv_dialects` (which exercises a real backward RV32I
  branch through the unrelated fixed-multiplier path) all still pass.
- **The instructions themselves:** `rel32_offset(target, own_length)` is exactly
  `sub(span(here(), target), own_length)`. Two format structs — `Rel32Instr`
  (`opcode: Byte` + the `rel32` field) for `jmp`/`call` (1 opcode byte,
  `own_length=5`) and `Rel32Instr2` (two opcode bytes) for the `jcc` family
  (`own_length=6`) — with `rel32: LittleEndian<Positioned<32>, 32>` in both: unlike
  Phase 2/3's immediates (always a plain, already-known `int` at macro-expansion
  time, splittable into little-endian `Byte`s directly via `byte_of`), `rel32_offset`
  stays a `Deferred` until `bitter` resolves it, so there's no concrete value yet
  for `byte_of` to split — `LittleEndian<T, width>` wraps the *whole* resolved
  32-bit value once `bitter` knows it and byte-reverses only that, leaving the
  preceding opcode byte(s) — already individual, order-fixed bytes — untouched
  (unlike RISC-V, which byte-reverses its one indivisible 32-bit word as a whole).
  All 16 `Jcc` condition codes (`tttn`, Intel SDM's own encoding, shared with the
  short `Jcc rel8`/`SETcc`/`CMOVcc` families this file doesn't implement) route
  through one internal `jcc_instr(condition, target)` helper.
**Files:** `bitter/src/pack.rs` (`resolve_span` generalized, two new tests, one
existing test rewritten from `span_rejects_a_from_greater_than_to` to
`span_returns_a_negative_distance_when_from_is_after_to`); `std/bitter/deferred.basm`
(`span`'s two new overloads, doc comment rewritten); `tests/fixtures/bitter/
deferred_dispatch.basm` (extended with `span` dispatch assertions); `std/x86_64/
impl.basm` (extended: `rel32_offset`, `Rel32Instr`/`Rel32Instr2`, `jmp`/`call`/
`jcc_instr`/all 16 `Jcc` mnemonics/`ret`); `tests/fixtures/x86_64/control_flow.basm`
(new — deliberately non-uniform instruction byte lengths between labels, one
forward branch and two backward branches, so both direction and real per-entry
byte-width-awareness are exercised, not just uniform-width arithmetic that would've
worked even with RISC-V's old fixed-multiplier trick); `tests/x86_64_encoding.rs`
(extended with `control_flow_encodes_correctly`).
**Verification:** `cargo test --test x86_64_encoding` passes (all three tests: the
existing `regdirect`/`regmem` plus the new `control_flow`, 8/8 emitted values, 30
bytes, matching hand-computed expected output exactly — hand-verified byte offsets
confirm the forward `jmp`'s `+3` and both backward branches' negative offsets,
`je`'s `-24` and `call`'s `-14`). Every remaining `Jcc` mnemonic's condition-code
byte spot-checked directly against actual compiler output (not just design intent)
against the Intel SDM's table.

### Phase 5 — Remaining core subset: shl/shr/sar, lea, push/pop ✅
**Deliverable:** `shl/shr/sar` (reg,imm8 via `0xC1 /digit`, and reg,cl via `0xD3
/digit`), `lea` (reg,mem — reuses Phase 3's `MemOperand` machinery, never
dereferences), `push`/`pop` (reg forms, reusing the "register number in low 3 opcode
bits + REX.B" pattern `mov reg,imm32` established in Phase 2).
**Resolved:**
- **Naming collision found and fixed (real gotcha, not hypothetical):** the reg,CL
  shift forms have no natural count parameter (CL is the only register the
  encoding permits there, so the macro signature is just `(rd, w)`, no separate
  register argument). Naming that form `shr(rd: Reg, w: int)` resolves to `(int,
  int)` — identical to `std.bitter.deferred`'s own `shr(a: int, b: int)`. An
  attempt to dodge this by importing only specific names from `std.bitter.deferred`
  (`from ... import sub, span, here, Deferred, Positioned`, leaving `shr` out) did
  **not** work: confirmed empirically that `bitterasm check` still reports
  `AmbiguousMacroOverload` for a bare `shr rbx, 0` call. Root cause, found in
  `src/loader.rs`'s `collect_declarations`: importing *any* name from a module
  recursively splices that module's *entire* transitive declaration set into the
  flattened program — a named import list is only a typo check against what's
  declared, not a visibility filter on what gets spliced. So `std.bitter.deferred`'s
  `shr` is unavoidably in scope for anything that imports `std.x86_64.impl`, no
  matter how `impl.basm` itself imports it. Fixed by naming the reg,CL forms
  `shl_cl`/`shr_cl`/`sar_cl` instead of overloading the plain mnemonic by arity —
  `shl_cl`/`sar_cl` don't actually collide with anything (`std.bitter.deferred` has
  no `shl`/`sar`), but use the same suffix anyway so all three mnemonics stay
  parallel rather than two overloading and one not. Reverted the import-list
  workaround (`from std.bitter.deferred import *` again) since it didn't help and
  the comment claiming it did would've been actively misleading to a future reader.
- `reg_imm_instr` (Phase 2) gained an explicit `imm_len` parameter (was hardcoded to
  `4`) so it could serve `shl`/`shr`/`sar`'s imm8 form (`imm_len=1`) and their `_cl`
  forms (`imm_len=0`, no immediate at all — the `imm` value passed is simply never
  read) alongside the existing ALU/`test` imm32 forms — all 7 existing call sites
  updated to pass `4` explicitly.
- `lea` reuses `mem_reg_instr` (Phase 3) with its two roles swapped from every other
  caller: `mem_reg_instr`'s `MemOperand` parameter is normally the actual
  destination (`mov [mem], rs` stores into it), but for `lea rd, mem` the
  destination is always the register `rd` (placed in ModRM.reg, same slot every
  other caller's `Reg` argument fills) while `mem` is the address *expression* —
  opcode `0x8D` (vs. `0x8B`, which this file doesn't implement — see Phase 3's own
  note on why the mov/ALU `MemOperand` overloads only ever cover the store
  direction) is the only thing distinguishing "compute this address" from "load
  from this address."
**Files:** `std/x86_64/impl.basm` (extended: `reg_imm_instr`'s `imm_len`
generalization plus its 7 existing callers updated; `shl`/`shr`/`sar` +
`shl_cl`/`shr_cl`/`sar_cl`; `lea`; `push`/`pop`); `tests/fixtures/x86_64/
shift_lea_stack.basm` (new, 12 cases); `tests/x86_64_encoding.rs` (extended with
`shift_lea_stack_encodes_correctly`).
**Verification:** `cargo test --test x86_64_encoding` passes (all four tests;
`shift_lea_stack` alone: 12/12 emitted values, 33 bytes, matching hand-computed
expected output exactly). This completes v1's instruction coverage.

### Phase 6 — Native (Intel-syntax) dialect ✅
**Deliverable:** `std/x86_64/native.basm`, giving every impl.basm macro real Intel
mnemonic syntax (`mov rax, rbx`, `add rax, 5`, `mov rax, [rbx+rcx*4+0x10]`), mirroring
RISC-V's `native.basm` sugar-injection approach.
**Resolved (three real findings, in the order they had to be worked through):**
- **Operand size:** every mnemonic hardcodes `w=1` (64-bit), dropping `impl.basm`'s
  explicit trailing `w` parameter from the surface syntax entirely. There is no
  32-bit register name family in this package (`rax`/`rbx`/... are the *only*
  spellings — unlike real x86-64, nothing here reads a 32-bit operand size off a
  different register name the way `eax` vs `rax` would), so a 32-bit form has no
  natural spelling to give it in a dialect. 32-bit forms stay fully reachable by
  writing `w` explicitly (`mov rax, rbx, 0`) — this file only *adds* syntax, it
  never removes `impl.basm`'s own default positional syntax.
- **`@emit`-only macros can't be called as expressions — delegate with a bare
  statement instead:** every `impl.basm` instruction macro (Phase 2 onward)
  `@emit`s, never `@return`s, so `@emit mov(rd, rs, 1)` inside a new wrapper's body
  is a hard error ("expected a value expression") — there's no value to emit, only
  a side effect to trigger. The fix is a bare, parenthesis-free statement instead:
  `mov rd, rs, 1` (no `@emit`, no parens — parenthesized call syntax is invalid at
  statement level, confirmed empirically the same way Phase 2 first hit it).
- **Claiming a name for one shape breaks every other shape's default syntax,
  including a wrapper's own internal delegation** — the deepest finding, and the
  reason nearly every mnemonic below has *two* registered patterns, not one: once
  any syntax exists for a name (`crate::parser::statements::parse_invocation_statement`'s
  "claimed" check), default positional syntax is gone for *every* arity of that
  name, not just the one the new syntax covers. A first draft that dropped `w` via
  a reduced-arity `mov` wrapper, with no restatement alongside it, broke that same
  wrapper's own internal `mov rd, rs, 1` delegation call — 3 operands no longer
  parses once `mov` only has a 2-operand pattern registered. Fixed by giving nearly
  every `w`-parameterized mnemonic a second, standalone `syntax NAME(a, b, c) =
  { NAME $a$, $b$, $c$ }` pattern that just restates the real underlying macro's
  own full signature verbatim (byte-for-byte what default syntax already produced)
  — needed purely so 3-argument calls (including this file's own delegating ones)
  keep parsing at all. Also confirmed empirically: the standalone restatement has
  to be declared *before* the inline-faceted reduced-arity macro in the same file,
  or the *first* attempt at combining them fails with a spurious parse error on the
  delegating call.
- **`mov`'s bracket-memory syntax needed a real compiler fix, not a workaround —
  see the two commits immediately before this phase's own instruction work
  ("Prefer the more literal-specific pattern..." and "add mov's load
  direction..."):** a `syntax` capture is an unbounded generic expression, stopped
  only by the next literal token in its *own* pattern, so `[$base$+$disp$]`'s
  `disp` capture happily swallows an indexed form's entire `$index$*$scale$+$disp$`
  right-hand side as one ordinary, well-typed expression — confirmed this hits
  *every* pair of the four addressing shapes (`[base]`/`[base+disp]`,
  `[base+disp]`/`[base+index*scale+disp]`, `[base+disp]`/`[rip+disp]`), including
  `PROGRESS.md`'s own flagship `mov rax, [rbx+rcx*4+0x10]` example, which was a hard
  "ambiguous syntax" error before the fix. Type resolution can't rescue this the
  way it does for two genuinely type-differentiated overloads sharing one pattern
  shape (`identical_syntax_overloads_defer_to_type_resolution`'s own case): both
  readings here are equally well-typed, they disagree on the source text's
  *structure*, not on which declared type accepts a shared, already-fixed operand
  list — and structure has to be settled before types even exist, since the parser
  can't hold multiple candidate shapes open while it waits for a later pass.
  Fixed in `src/parser/invocation_syntax.rs`: prefer whichever successful match
  used the *most* literal (non-capture) tokens, instead of erroring, whenever more
  than one candidate parses the same text; two genuinely equally-specific patterns
  (like RISC-V's own `$a$ + $b$` vs. `$b$ + $a$`) are untouched and still hit the
  original ambiguity error (verified against both pre-existing tests that depend on
  that, plus a new one for the rescued case, plus the full existing test suite
  staying green). This also retroactively resolves Phase 5's own `shl_cl`/`shr_cl`/
  `sar_cl` "no natural collision-free spelling" compromise — real `shl rax, cl`
  syntax (a literal `cl` token vs. a generic `$imm$` capture) is the same category
  of ambiguity, now handled the same way. `mov`'s load direction (`0x8B`) didn't
  exist before this phase either — Phase 3 deliberately covered only the store
  direction — added as its own small, real-overloading addition to `impl.basm`
  (distinguished from the store overload by its first parameter's type, `Reg` vs.
  `MemOperand`, no naming trick needed), verified byte-exact by extending Phase 3's
  own `regmem.basm`/`regmem_encodes_correctly`.
- **Two more `std.bitter.deferred` naming collisions, same root cause as Phase 5's
  `shl_cl`/`shr_cl`/`sar_cl`:** the ALU `sub` and shift `shr` reg,imm reduced-arity
  wrappers (`sub(rd: Reg, rs: Reg)`, `shr(rd: Reg, imm: int)`) each resolve to
  `(int, int)`, colliding with `std.bitter.deferred`'s own `sub(int, int)`/
  `shr(int, int)` (always transitively in scope — see Phase 5's note on why naming
  an import doesn't limit what's spliced). Fixed the same way: unique internal
  names (`sub2`, `shr_imm`) with an unanchored `syntax` pattern still spelling the
  real mnemonic (`sub`, `shr`) at the call site.
- **reg,reg vs. reg,imm still separate mnemonics:** `movi`/`addi`/.../`testi` keep
  `impl.basm`'s own `i`-suffixed names rather than merging into `mov`/`add`/... —
  same `Reg`-is-a-plain-`int` reasoning Phase 2 already resolved, restated here
  since real Intel syntax spells both forms identically and a reader might
  reasonably expect this dialect to paper over that; it can't, for the same reason
  Phase 2 couldn't. Memory-operand forms don't have this problem (a bracketed
  operand is lexically distinct from a bare one), which is why `mov`'s many memory
  shapes safely share its name while its reg,imm sibling can't share `mov`'s own.
- **Scope kept deliberately narrower than "every instruction" for bracket sugar:**
  full register *and* memory-operand (all four addressing shapes, both directions)
  syntax is built for `mov` (the phase's own flagship example) and `lea` (address
  syntax is its entire purpose). The other six ALU ops and `test` get full
  register-only sugar (2-argument reduced plus 3-argument restatement) but no
  dedicated memory-bracket forms — the exact same mechanical pattern `mov`'s store
  direction demonstrates would apply unchanged; `impl.basm`'s own `Mem(...)`/
  `MemIndexed(...)`/`MemRipRelative(...)` constructor calls remain directly usable
  as any of their second operands regardless (ordinary expressions, needing no
  dialect support to already work), so this is a trivial, mechanical extension left
  undone rather than a real capability gap, the same reduced-scope-now precedent
  used repeatedly already in this file (`span`'s missing `(Deferred, Deferred)`,
  `shr`/`band`'s int-only args).
**Files:** `src/parser/invocation_syntax.rs` (literal-specificity tie-breaking,
separate commit); `std/x86_64/impl.basm` (`mov`'s load direction, separate commit);
`std/x86_64/native.basm` (new); `tests/fixtures/x86_64/dialect_native.basm` (new,
exercises every family of syntax sugar the dialect adds) and `tests/fixtures/x86_64/
dialect_default.basm` (new, the identical instructions in `impl.basm`'s own default
syntax); `tests/x86_64_dialects.rs` (new — adapted from `tests/riscv_dialects.rs`'s
two-dialect comparison to a dialect-vs-default one, since x86-64 only has one
dialect so far).
**Verification:** `cargo test --test x86_64_dialects` passes (39/39 emitted values,
176 bytes, byte-identical between the two fixtures). 18 representative instructions
additionally spot-checked by hand against the Intel SDM before the fixture was
written. Full existing suite (`cargo test --lib`, every other `tests/*.rs`) still
green.

### Phase 7 — Independent-oracle cross-check (GNU binutils)
**Deliverable:** `tests/x86_64/` mirroring `tests/riscv/`'s harness (`run_tests.py`,
`docker/Dockerfile` + `assemble.sh`, paired `cases/<name>.s` / `<name>.basm`
fixtures) — likely *simpler* than RISC-V's version since GNU binutils' `as`/`objdump`
target x86-64 natively (no cross-toolchain package needed, and a system `as` may
already suffice without Docker — check before assuming Docker is required).
**Files:** `tests/x86_64/` (new: `run_tests.py`, `docker/`, `cases/`).
**Verification:** running the harness catches any Intel-SDM transcription mistakes
made while hand-verifying Phases 2-5's expected bytes.
