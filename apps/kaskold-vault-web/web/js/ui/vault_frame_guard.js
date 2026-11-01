// A meta CSP cannot carry frame-ancestors, and static hosts may not send it as a
// header, so the Vault refuses to run its UI inside another page's frame.
export function isFramed(view) {
  return view.top !== view.self;
}

export function refuseFramedVault(view, root) {
  if (!isFramed(view)) return false;
  root.replaceChildren();
  return true;
}
