// @ts-check
/**
 * Resolve an element by id without routing a pure lookup through app state.
 *
 * @param {string} id
 */
export function byId(id) {
    return document.getElementById(id);
}

/**
 * Bind an optional click target without letting removed UI abort sibling bindings.
 *
 * @param {string} id
 * @param {GlobalEventHandlers['onclick']} handler
 */
export function bindClick(id, handler) {
    const target = byId(id);
    if (target) target.onclick = handler;
    return target;
}

/**
 * Bind an optional change target without letting removed UI abort sibling bindings.
 *
 * @param {string} id
 * @param {GlobalEventHandlers['onchange']} handler
 */
export function bindChange(id, handler) {
    const target = byId(id);
    if (target) target.onchange = handler;
    return target;
}

/**
 * Bind an optional key target without letting removed UI abort sibling bindings.
 *
 * @param {string} id
 * @param {GlobalEventHandlers['onkeydown']} handler
 */
export function bindKeydown(id, handler) {
    const target = byId(id);
    if (target) target.onkeydown = handler;
    return target;
}
