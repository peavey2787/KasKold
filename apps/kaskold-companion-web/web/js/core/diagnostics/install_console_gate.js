// Imported first by main.js so the gate is in place before any module logs.
import { gateConsole } from './console_gate.js';

gateConsole(globalThis.console, globalThis.localStorage);
