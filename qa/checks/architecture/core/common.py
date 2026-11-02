"""Shared parsers used by multiple architecture checks."""

from __future__ import annotations

import re
from pathlib import Path, PurePath


IGNORED_TREE_PARTS = {
    ".git",
    ".pytest_cache",
    "__pycache__",
    "node_modules",
    "target",
    "vendor",
    "generated",
    "dist",
}


def relative_posix(path: PurePath, root: PurePath) -> str:
    """Return a repository-relative path with stable POSIX separators."""
    return path.relative_to(root).as_posix()


def has_exact_child(directory: Path, name: str) -> bool:
    """Return whether a directory contains an entry with exactly this spelling."""
    try:
        return any(entry.name == name for entry in directory.iterdir())
    except OSError:
        return False


GENERATED_PREFIXES = (
    Path("apps/kaskold-companion-web/web/pkg"),
    Path("apps/kaskold-vault-web/web/pkg"),
    Path("apps/kaskold-companion-web/web/lib"),
    Path("apps/kaskold-vault-web/web/lib"),
    Path("apps/kaskold-companion-android/.gradle"),
    Path("apps/kaskold-companion-android/.kotlin"),
    Path("apps/kaskold-companion-android/build"),
    Path("apps/kaskold-companion-android/app/build"),
    Path("apps/kaskold-vault-android/.gradle"),
    Path("apps/kaskold-vault-android/.kotlin"),
    Path("apps/kaskold-vault-android/app/.cxx"),
    Path("apps/kaskold-vault-android/app/build"),
)


def is_generated_tree(path: Path, root: Path) -> bool:
    """Return whether a path is generated or dependency-owned build output."""
    try:
        relative = path.relative_to(root)
    except ValueError:
        relative = path
    if any(relative == prefix or prefix in relative.parents for prefix in GENERATED_PREFIXES):
        return True
    return any(part in IGNORED_TREE_PARTS for part in relative.parts)

def _rust_char_literal_end(source: str, index: int) -> int | None:
    """Return a char literal's closing quote, without mistaking lifetimes for chars."""
    cursor = index + 1
    if cursor >= len(source) or source[cursor] in "\r\n'":
        return None
    if source[cursor] == "\\":
        cursor += 1
        if cursor >= len(source):
            return None
        escape = source[cursor]
        if escape == "x":
            cursor += 3
        elif escape == "u" and cursor + 1 < len(source) and source[cursor + 1] == "{":
            closing = source.find("}", cursor + 2)
            if closing == -1:
                return None
            cursor = closing + 1
        else:
            cursor += 1
    else:
        cursor += 1
    return cursor if cursor < len(source) and source[cursor] == "'" else None

def _rust_raw_string_start(source: str, index: int) -> tuple[int, int] | None:
    """Return `(opening_quote, hash_count)` for `r#"..."#` and byte variants."""
    cursor = index
    if source.startswith("br", cursor):
        cursor += 1
    if cursor >= len(source) or source[cursor] != "r":
        return None
    cursor += 1
    hash_start = cursor
    while cursor < len(source) and source[cursor] == "#":
        cursor += 1
    if cursor < len(source) and source[cursor] == '"':
        return cursor, cursor - hash_start
    return None

class _ScanState:
    """Mutable cursor state shared by the Rust and brace-body source scanners."""

    def __init__(self) -> None:
        self.mode = "code"
        self.close = ""
        self.escaped = False
        self.block_depth = 0
        self.raw_hashes = 0
        self.depth = 0
        self.parens = 0
        self.brackets = 0
        self.done = False
        self.result: int | None = None

def _next_char(source: str, index: int) -> str:
    return source[index + 1] if index + 1 < len(source) else ""

def _enter_quoted(state: _ScanState, close: str) -> None:
    state.mode = "quoted"
    state.close = close

def _enter_comment(source: str, index: int, state: _ScanState) -> int | None:
    char = source[index]
    following = _next_char(source, index)
    if char == "/" and following == "/":
        state.mode = "line_comment"
        return index + 1
    if char == "/" and following == "*":
        state.mode = "block_comment"
        state.block_depth = 1
        return index + 1
    return None

def _enter_literal(
    source: str,
    index: int,
    state: _ScanState,
    *,
    javascript: bool,
    rust: bool,
) -> int | None:
    """Enter a string, char, raw string or comment; return the index to resume from."""
    raw = _rust_raw_string_start(source, index) if rust else None
    if raw is not None:
        opening_quote, state.raw_hashes = raw
        state.mode = "raw_string"
        return opening_quote
    char = source[index]
    if char == '"':
        _enter_quoted(state, '"')
        return index
    if char == "'" and (javascript or (rust and _rust_char_literal_end(source, index) is not None)):
        _enter_quoted(state, "'")
        return index
    if javascript and char == "`":
        _enter_quoted(state, "`")
        return index
    return _enter_comment(source, index, state)

def _quoted_step(char: str, state: _ScanState) -> None:
    if state.escaped:
        state.escaped = False
    elif char == "\\":
        state.escaped = True
    elif char == state.close:
        state.mode = "code"

def _raw_string_step(source: str, index: int, state: _ScanState) -> int:
    terminator = '"' + "#" * state.raw_hashes
    if source.startswith(terminator, index):
        state.mode = "code"
        return index + len(terminator) - 1
    return index

def _block_comment_step(source: str, index: int, state: _ScanState) -> int:
    char = source[index]
    following = _next_char(source, index)
    if char == "/" and following == "*":
        state.block_depth += 1
        return index + 1
    if char == "*" and following == "/":
        state.block_depth -= 1
        if state.block_depth == 0:
            state.mode = "code"
        return index + 1
    return index

def _scan_literal_step(source: str, index: int, state: _ScanState) -> int:
    """Advance one character inside a string, raw string or comment."""
    char = source[index]
    if state.mode == "quoted":
        _quoted_step(char, state)
    elif state.mode == "raw_string":
        return _raw_string_step(source, index, state)
    elif state.mode == "line_comment":
        if char == "\n":
            state.mode = "code"
    else:
        return _block_comment_step(source, index, state)
    return index

def _run_scan(source: str, start: int, code_step) -> int | None:
    """Scan from `start` until `code_step` records a result; None when the source ends first."""
    state = _ScanState()
    index = start
    while index < len(source):
        if state.mode == "code":
            index = code_step(source, index, state)
        else:
            index = _scan_literal_step(source, index, state)
        if state.done:
            return state.result
        index += 1
    return None

def _rust_declaration_code_step(source: str, index: int, state: _ScanState) -> int:
    opened = _enter_literal(source, index, state, javascript=False, rust=True)
    if opened is not None:
        return opened
    char = source[index]
    if char == "(":
        state.parens += 1
    elif char == ")":
        state.parens = max(0, state.parens - 1)
    elif char == "[":
        state.brackets += 1
    elif char == "]":
        state.brackets = max(0, state.brackets - 1)
    elif char == "{" and state.parens == 0 and state.brackets == 0:
        state.done, state.result = True, index
    elif char == ";" and state.parens == 0 and state.brackets == 0:
        state.done, state.result = True, None
    return index

def _balanced_code_step(
    source: str,
    index: int,
    state: _ScanState,
    *,
    javascript: bool,
    rust: bool,
) -> int:
    opened = _enter_literal(source, index, state, javascript=javascript, rust=rust)
    if opened is not None:
        return opened
    char = source[index]
    if char == "{":
        state.depth += 1
    elif char == "}":
        state.depth -= 1
        if state.depth == 0:
            state.done, state.result = True, index
    return index

def _rust_body_opening(source: str, start: int) -> int | None:
    """Find a Rust function body without treating `[T; N]` as a declaration end."""
    return _run_scan(source, start, _rust_declaration_code_step)

def balanced_body_closing(
    source: str,
    opening: int,
    *,
    javascript: bool = False,
    rust: bool = False,
) -> int | None:
    """Return a balanced body's closing brace for supported source languages."""
    return _run_scan(
        source,
        opening,
        lambda text, index, state: _balanced_code_step(
            text, index, state, javascript=javascript, rust=rust
        ),
    )

def _blank(output: list[str], start: int, stop: int) -> None:
    for cursor in range(start, stop):
        output[cursor] = " "

def _mask_code_step(source: str, index: int, output: list[str], state: _ScanState) -> int:
    raw = _rust_raw_string_start(source, index)
    if raw is not None:
        opening_quote, state.raw_hashes = raw
        _blank(output, index, opening_quote + 1)
        state.mode = "raw_string"
        return opening_quote
    char = source[index]
    following = _next_char(source, index)
    if char == '"':
        _blank(output, index, index + 1)
        _enter_quoted(state, '"')
    elif char == "'" and _rust_char_literal_end(source, index) is not None:
        _blank(output, index, index + 1)
        _enter_quoted(state, "'")
    elif char == "/" and following == "/":
        _blank(output, index, index + 2)
        state.mode = "line_comment"
        return index + 1
    elif char == "/" and following == "*":
        _blank(output, index, index + 2)
        state.mode = "block_comment"
        state.block_depth = 1
        return index + 1
    return index

def _mask_raw_string_step(source: str, index: int, output: list[str], state: _ScanState) -> int:
    if source[index] != "\n":
        output[index] = " "
    terminator = '"' + "#" * state.raw_hashes
    if source.startswith(terminator, index):
        _blank(output, index, min(len(source), index + len(terminator)))
        state.mode = "code"
        return index + len(terminator) - 1
    return index

def _mask_block_comment_step(source: str, index: int, output: list[str], state: _ScanState) -> int:
    char = source[index]
    following = _next_char(source, index)
    if char != "\n":
        output[index] = " "
    if char == "/" and following == "*":
        output[index + 1] = " "
        state.block_depth += 1
        return index + 1
    if char == "*" and following == "/":
        output[index + 1] = " "
        state.block_depth -= 1
        if state.block_depth == 0:
            state.mode = "code"
        return index + 1
    return index

def _mask_literal_step(source: str, index: int, output: list[str], state: _ScanState) -> int:
    char = source[index]
    if state.mode == "raw_string":
        return _mask_raw_string_step(source, index, output, state)
    if state.mode == "block_comment":
        return _mask_block_comment_step(source, index, output, state)
    if char != "\n":
        output[index] = " "
    if state.mode == "quoted":
        _quoted_step(char, state)
    elif char == "\n":
        state.mode = "code"
    return index

def rust_code_only(source: str) -> str:
    """Mask Rust comments and literals while preserving code positions and lines."""
    output = list(source)
    state = _ScanState()
    index = 0
    while index < len(source):
        if state.mode == "code":
            index = _mask_code_step(source, index, output, state)
        else:
            index = _mask_literal_step(source, index, output, state)
        index += 1
    return "".join(output)

def _top_level_parts(source: str) -> list[str]:
    """Split a Rust use-tree group on commas outside nested groups."""
    parts: list[str] = []
    start = 0
    depth = 0
    for index, char in enumerate(source):
        if char == "{":
            depth += 1
        elif char == "}":
            depth = max(0, depth - 1)
        elif char == "," and depth == 0:
            parts.append(source[start:index].strip())
            start = index + 1
    tail = source[start:].strip()
    if tail:
        parts.append(tail)
    return parts

def _use_group_bounds(tree: str) -> tuple[int, int] | None:
    opening = tree.find("{")
    if opening < 0:
        return None
    depth = 0
    for index in range(opening, len(tree)):
        if tree[index] == "{":
            depth += 1
        elif tree[index] == "}":
            depth -= 1
            if depth == 0:
                return opening, index
    return None

def _join_rust_path(prefix: str, suffix: str) -> str:
    left = prefix.strip().strip(":")
    right = suffix.strip().strip(":")
    if right == "self":
        return left
    if not left:
        return right
    if not right:
        return left
    return f"{left}::{right}"

def _expand_rust_use_tree(tree: str, prefix: str = "") -> set[str]:
    tree = re.sub(r"\s+as\s+[A-Za-z_][A-Za-z0-9_]*\s*$", "", tree.strip())
    bounds = _use_group_bounds(tree)
    if bounds is None:
        path = _join_rust_path(prefix, tree)
        return {path} if path else set()
    opening, closing = bounds
    base = tree[:opening].rstrip().removesuffix("::")
    group_prefix = _join_rust_path(prefix, base)
    paths: set[str] = set()
    for member in _top_level_parts(tree[opening + 1:closing]):
        paths.update(_expand_rust_use_tree(member, group_prefix))
    suffix = tree[closing + 1:].strip()
    if suffix:
        paths = {_join_rust_path(path, suffix) for path in paths}
    return paths

def rust_use_paths(source: str) -> set[str]:
    """Expand direct and grouped Rust `use` declarations into full paths."""
    code = rust_code_only(source)
    paths: set[str] = set()
    for match in re.finditer(r"\buse\s+(?P<tree>[^;]+);", code, re.DOTALL):
        paths.update(_expand_rust_use_tree(match.group("tree")))
    return paths

def _rust_body_closing(source: str, opening: int) -> int | None:
    return balanced_body_closing(source, opening, rust=True)

def rust_function_lengths(
    source: str,
    *,
    include_indented: bool = False,
) -> list[tuple[str, int]]:
    """Return Rust function names and source lengths using balanced signatures/bodies."""
    indent = r"[ \t]*" if include_indented else ""
    pattern = re.compile(
        rf"(?m)^{indent}(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?"
        r"(?:unsafe\s+)?(?:extern\s+\"[^\"]+\"\s+)?fn\s+"
        r"([A-Za-z_][A-Za-z0-9_]*)"
    )
    functions: list[tuple[str, int]] = []
    for match in pattern.finditer(source):
        opening = _rust_body_opening(source, match.end())
        if opening is None:
            continue
        closing = _rust_body_closing(source, opening)
        if closing is not None:
            functions.append(
                (match.group(1), source[match.start():closing + 1].count("\n") + 1)
            )
    return functions
