# rclox

Bytecode VM for the Lox language in Rust. Follows *Crafting Interpreters* Part III (clox) by Robert Nystrom, with some extensions.

---

## Design

### Pipeline

```
Source text
  └─ Scanner (src/scanner.rs)
       │  tokenises lazily — no token Vec allocated upfront
       └─ Parser (src/compiler/parser.rs)
            │  one token of lookahead (current + previous)
            │  panic-mode error recovery: suppress cascading errors until
            │  next statement boundary (semicolon or statement keyword)
            └─ Compiler (src/compiler/mod.rs)
                 │  Pratt parser — no AST, emits bytecode directly
                 │  stack of FunctionCompiler frames (one per fun { })
                 └─ Chunk (src/chunk.rs)
                      │  bytecode + constant pool + line table
                      └─ VM (src/vm.rs)
                           stack-based interpreter
```

No intermediate representation. The compiler is a recursive descent / Pratt parser that emits opcodes as it parses. The entire pipeline runs in a single pass over the source.

---

### Compiler

**Pratt parser** (`src/compiler/rules.rs`)

Each `TokenType` maps to a `ParseRule { prefix, infix, precedence }`. `parse_precedence(min)` drives the loop:
1. Consume the next token; call its `prefix` handler (literals, unary, grouping, variable, `switch`, prefix `++`/`--`).
2. While the next token's precedence ≥ `min`, consume and call its `infix` handler (binary ops, `and`, `or`, call, postfix `++`/`--`).

This correctly handles precedence and associativity without explicit grammar rules.

**FunctionCompiler frame stack** (`src/compiler/frame.rs`)

Each `fun` pushes a new `FunctionCompiler` onto `Compiler::frames`. It owns:
- `locals: Vec<Local>` — slot 0 is a synthetic empty-name local reserving the function's own stack slot; user locals start at 1.
- `scope_depth` — incremented on `{`, decremented on `}`.
- `jumps: Vec<(loop_depth, continue_target, Vec<break_patch>)>` — tracks pending `break`/`continue` patches per loop nesting level.
- `function: *mut Object` — the heap-allocated `FunctionObject` being filled.

`end_compiler()` pops the frame and returns the completed function pointer as a `Value::Object` constant, which the enclosing chunk loads with `OpConstant`. Functions are values.

**Local variable resolution**

`resolve_local` scans `locals` in reverse to find the innermost binding. Uninitialised locals (declared but not yet past their initialiser) are skipped with an error, preventing `var x = x;`. If not found locally, falls back to a global by name (emits `OpGetGlobal`/`OpSetGlobal` with the name as a constant-pool string).

**Scope exit**

`end_scope` / `discard_locals` counts how many locals are deeper than the target depth and emits either:
- `OpPopN n` — discard n values (normal scope exit)
- `OpYield n` — preserve the top value, pop the n locals beneath it (used by `switch` case blocks to return a value out of a scope)

**Jump patching**

Jumps emit `0xff 0xff` as placeholder offsets, returning the patch site. `patch_jump` backfills the real 16-bit relative offset once the target address is known. `emit_loop` emits `OpLoop` with a backwards offset back to loop start. `break` and `continue` patch their respective targets at loop end / loop top.

---

### Chunk

```
Chunk {
    code:           Vec<u8>         // bytecode
    constants:      Vec<Value>      // constant pool (max 256 entries per chunk)
    lines:          Vec<usize>      // parallel to code, one line per byte
    constant_index: HashMap<(u8,u64), u8>  // dedup: (tag, bits) → pool index
}
```

Constant deduplication keyed on `(type_tag, f64::to_bits | ptr as u64)` — identical number literals or already-interned string pointers reuse the same constant slot.

The `Debug` impl is a full disassembler: walks the bytecode and pretty-prints every instruction with offset, line, opcode, and operand.

---

### Value & Object system

```
Value
  ├─ Nil
  ├─ Bool(bool)
  ├─ Number(f64)
  └─ Object(*mut Object)        // raw pointer into the Heap

Object
  ├─ obj_type: ObjectType
  │    ├─ String(String)
  │    ├─ Function(FunctionObject)
  │    ├─ Native(NativeFunction)
  │    ├─ Array(Vec<Value>)
  │    ├─ Closure(ClosureObject)   // runtime wrapper around Function; holds upvalue slots
  │    └─ UpValue(UpValueObject)   // open: location → stack slot; closed: location → self.closed
  ├─ is_marked: bool            // for future mark-and-sweep GC
  └─ next: *mut Object          // intrusive linked list through the Heap
```

`Value` is `Clone` — numbers/bools copy by value, `Object` copies the pointer. Equality on objects compares pointers (reference equality), which works correctly for strings because of interning.

---

### Heap

```
Heap { objects: *mut Object }  // head of the intrusive linked list
```

`allocate` boxes the object, calls `Box::into_raw`, and prepends it to the list. `Drop` walks the list and reconstructs `Box::from_raw` to free each node. All heap allocation goes through `Vm` helper methods (`allocate_string`, `allocate_function`, `allocate_object`) so the VM controls the list head.

**String interning** (`Vm::interned_strings: HashMap<String, *mut Object>`): `allocate_string` checks the map before allocating. Duplicate strings get the same pointer, so `==` on string objects is a cheap pointer compare.

---

### VM

```
Vm {
    stack:            Vec<Value>               // value stack (pre-allocated 256 cap — upvalue ptrs into it must not invalidate)
    heap:             Heap
    interned_strings: HashMap<String, *mut Object>
    globals:          HashMap<String, Value>
    call_stack:       Vec<CallFrame>
    open_upvalues:    Vec<*mut Object>         // all live UpValueObjects still pointing into the stack
}

CallFrame {
    closure:    *mut ClosureObject
    ip:         usize                // index into closure.function.chunk.code
    stack_base: usize                // start of this frame's locals in Vm::stack
}
```

`run()` is the main dispatch loop. It reads opcodes via `read_byte()` which advances `ip` on the current frame. Locals are accessed as `stack[stack_base + slot]` — no separate locals array.

**Call protocol**
1. Caller pushes the closure value, then all arguments.
2. `OpCall arg_count` resolves the value at `stack[len - arg_count - 1]`.
3. For `ObjectType::Closure`: push a new `CallFrame` with `stack_base = stack.len() - arg_count - 1`. The function's locals live in-place on the stack starting at `stack_base`.
4. For `ObjectType::Native`: slice args directly off the stack, call the function pointer, truncate stack, push result — no `CallFrame` needed.
5. `OpReturn`: close all upvalues pointing into this frame (`close_upvalues(stack[stack_base])`), pop `CallFrame`, truncate stack to `frame.stack_base`, push return value. If call stack is empty, program ends.

**Error handling**: `runtime_error` prints the message then walks `call_stack` in reverse to print a stack trace with file/function name and line number, then clears both stack and call stack so the REPL can continue.

---

### Opcode set

| Opcode | Operands | Effect |
|---|---|---|
| `OpConstant` | `u8` index | push `constants[index]` |
| `OpNil` / `OpTrue` / `OpFalse` | — | push literal |
| `OpPop` | — | pop 1 |
| `OpPopN` | `u8` n | pop n |
| `OpDup` | — | duplicate top |
| `OpYield` | `u8` n | save top, pop n beneath it, push saved |
| `OpNegate` / `OpNot` | — | unary on top |
| `OpAdd` / `OpSubtract` / `OpMultiply` / `OpDivide` | — | binary on top 2 |
| `OpEqual` / `OpGreater` / `OpLess` | — | comparison → bool |
| `OpPrint` | — | pop + println |
| `OpDefineGlobal` | `u8` name-constant | pop → globals |
| `OpGetGlobal` / `OpSetGlobal` | `u8` name-constant | globals r/w |
| `OpGetLocal` / `OpSetLocal` | `u8` slot | stack[base+slot] r/w |
| `OpJump` | `u16` offset | ip += offset |
| `OpJumpIfFalse` | `u16` offset | ip += offset if top is falsey |
| `OpLoop` | `u16` offset | ip -= offset |
| `OpCall` | `u8` arg_count | call top-of-stack closure or native |
| `OpReturn` | — | close upvalues in frame, return top value to caller |
| `OpClosure` | `u8` idx, then `(u8 is_local, u8 slot)*` | wrap function constant in a closure, capture upvalues |
| `OpGetUpvalue` / `OpSetUpvalue` | `u8` slot | read/write upvalue slot in current closure |
| `OpCloseUpvalue` | — | copy top-of-stack into its upvalue's `closed` field, pop |
| `OpArray` | `u8` n | pop n values, heap-allocate array, push |
| `OpMakeArray` | — | pop length n, heap-allocate nil array of size n, push |
| `OpGetIndex` | — | pop index + array, push element |
| `OpSetIndex` | — | pop value + index + array, mutate, push value |
| `OpLen` | — | pop array, push length as number |

---

### Extensions beyond the book

**`switch` expression** — expression-oriented: each `case` is a scoped block that `yield`s a value. The `yield` keyword inside a case saves the value across `end_scope` via `OpYield`. Compiles to a chain of `OpDup` / `OpEqual` / `OpJumpIfFalse` comparisons, O(n) cases.

**Prefix and postfix `++`/`--`** — resolved to local/global get+set pairs with `OpDup` to preserve the pre-increment value for postfix semantics.

**`break` and `continue`** — tracked per loop in `FunctionCompiler::jumps`. `break` emits a forward `OpJump` patch site collected at loop end. `continue` emits a backward `OpLoop` directly to the continue target (before the increment clause in `for`). Both call `discard_locals` to clean up any locals declared inside the loop body.

**Arrays** — `[1, "hello", true]` literal syntax and `array(n)` for nil-filled pre-allocation. Index get `arr[i]` and set `arr[i] = x` are proper l-values. `len(arr)` returns element count. Heterogeneous — elements are `Value`, any type mix is valid. Bounds and type checked at runtime. Arrays are heap objects, freed at VM teardown with everything else.

**Native function extension point** — `NativeFunction { arity, name, is_variadic, fun: fn(&[Value]) -> Value }`. New natives register via `get_native_functions()`. Currently: `clock()` (Unix time as f64), `floor(n)`, `mod(a, b)`.

**Closures / upvalues** — every function is wrapped in a `ClosureObject` at runtime. Captured variables become `UpValueObject`s — open upvalues hold a pointer into the stack; on scope exit or return they are closed (value copied into the object itself). Deduplication ensures two closures capturing the same local share one upvalue. Supports arbitrarily deep capture chains (inner → middle → outer).

---

## TODO (book chapters remaining)

- [ ] **Garbage collection** — tri-color mark-and-sweep; `is_marked` on `Object` already present; need GC roots (stack + globals + open upvalues) and a `gray_stack` worklist (ch. 26)
- [ ] **Classes and instances** — `ObjClass`, `ObjInstance`, `OpGetProperty`/`OpSetProperty` with field hash map (ch. 27)
- [ ] **Methods and `this`** — `ObjBoundMethod`, `OpInvoke` fast path, implicit `this` as slot 0 (ch. 28)
- [ ] **Inheritance** — `<` syntax, `ObjClass::superclass`, `OpGetSuper`/`OpInvokeSuper`, `super` keyword (ch. 29)

## Examples

**Fibonacci (recursion + timing)**
```lox
fun fib(n) {
  if (n < 2) return n;
  return fib(n - 1) + fib(n - 2);
}

var start = clock();
print fib(30);
print clock() - start;
```

**Closures / higher-order functions**
```lox
fun makeCounter(start) {
  var count = start;
  fun increment() {
    count = count + 1;
    print count;
  }
  return increment;
}

var c = makeCounter(0);
c();  // 1
c();  // 2
c();  // 3
```

**Switch expression**
```lox
fun grade(score) {
  return switch (floor(score / 10)) {
    case 10 => { yield "A+"; }
    case 9  => { yield "A";  }
    case 8  => { yield "B";  }
    default => { yield "F";  }
  };
}

print grade(95);  // A
print grade(72);  // F
```

**Loops with break / continue**
```lox
var i = 0;
while (true) {
  i++;
  if (mod(i, 2) == 0) continue;
  if (i > 9) break;
  print i;  // 1 3 5 7 9
}

for (var j = 0; j < 5; j++) {
  print j;
}
```

**Scoping and shadowing**
```lox
var x = "global";
{
  var x = "inner";
  print x;   // inner
}
print x;     // global
```

**Arrays**
```lox
fun reverse(arr) {
  var lo = 0;
  var hi = len(arr) - 1;
  while (lo < hi) {
    var tmp = arr[lo];
    arr[lo] = arr[hi];
    arr[hi] = tmp;
    lo++; hi--;
  }
}

fun indexOf(arr, val) {
  for (var i = 0; i < len(arr); i++) {
    if (arr[i] == val) return i;
  }
  return -1;
}

var a = [5, 3, 1, 4, 2];
reverse(a);
print a;              // [2, 4, 1, 3, 5]
print indexOf(a, 3);  // 2
```

**Native functions**
```lox
print clock();           // Unix epoch seconds as f64
print mod(17, 5);     // 2
print floor(9.7);     // 9
```

---

## Building & running

```
cargo build --release

./target/release/rclox             # REPL (vi keybindings, history via rustyline)
./target/release/rclox script.lox  # run file
#   exit 0  → ok
#   exit 65 → compile error
#   exit 70 → runtime error
```
