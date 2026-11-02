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
 * An element the page always contains; a missing one is a broken page.
 *
 * @param {string} id
 * @returns {HTMLElement}
 */
export function elementById(id) {
    const element = byId(id);
    if (!element) throw new Error(`Missing page element: ${id}`);
    return element;
}

/**
 * A form control the page always contains.
 *
 * @param {string} id
 * @returns {HTMLInputElement}
 */
export function inputById(id) {
    return /** @type {HTMLInputElement} */ (elementById(id));
}

/**
 * A form control the page may not be showing.
 *
 * @param {string} id
 * @returns {HTMLInputElement | null}
 */
export function optionalInput(id) {
    return /** @type {HTMLInputElement | null} */ (byId(id));
}

/**
 * The checkboxes inside a list.
 *
 * @param {ParentNode} container
 * @returns {NodeListOf<HTMLInputElement>}
 */
export function checkboxesIn(container) {
    return container.querySelectorAll('input[type="checkbox"]');
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
