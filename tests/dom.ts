import { Window } from "happy-dom";
const browser = new Window({ url: "http://localhost" });
Object.assign(globalThis, {
  window: browser,
  document: browser.document,
  navigator: browser.navigator,
  HTMLElement: browser.HTMLElement,
  HTMLInputElement: browser.HTMLInputElement,
  HTMLButtonElement: browser.HTMLButtonElement,
  SVGElement: browser.SVGElement,
  Element: browser.Element,
  Node: browser.Node,
  Document: browser.Document,
  DocumentFragment: browser.DocumentFragment,
  MutationObserver: browser.MutationObserver,
  ResizeObserver: browser.ResizeObserver,
  DOMRect: browser.DOMRect,
  Event: browser.Event,
  MouseEvent: browser.MouseEvent,
  PointerEvent: browser.PointerEvent,
  KeyboardEvent: browser.KeyboardEvent,
  getComputedStyle: browser.getComputedStyle.bind(browser),
  requestAnimationFrame: browser.requestAnimationFrame.bind(browser),
  cancelAnimationFrame: browser.cancelAnimationFrame.bind(browser),
  matchMedia: browser.matchMedia.bind(browser),
  IS_REACT_ACT_ENVIRONMENT: true,
});
browser.HTMLElement.prototype.getBoundingClientRect = function () {
  return new browser.DOMRect(30, 30, 260, 40);
};
// Happy DOM's partial Web Animations API rejects canceled animations differently
// from Chromium. Exercise Motion's supported JavaScript animation path instead.
delete browser.Element.prototype.animate;
