/** Resolve an element by id without routing a pure lookup through app state. */
export function byId(id) {
    return document.getElementById(id);
}

/** Bind an optional click target without letting removed UI abort sibling bindings. */
export function bindClick(id, handler) {
    const target = byId(id);
    if (target) target.onclick = handler;
    return target;
}

/** Bind an optional change target without letting removed UI abort sibling bindings. */
export function bindChange(id, handler) {
    const target = byId(id);
    if (target) target.onchange = handler;
    return target;
}

/** Bind an optional key target without letting removed UI abort sibling bindings. */
export function bindKeydown(id, handler) {
    const target = byId(id);
    if (target) target.onkeydown = handler;
    return target;
}
