"use strict";

((window) => {
    const bootstrap = window.__bootstrap;
    const {
        ObjectAssign
    } = bootstrap.primordials;

    const isolator = bootstrap.isolator;

    const base64 = bootstrap.base64;
    const timers = bootstrap.timers;

    const core = Deno.core
    let hasBootstrapped = false;

    function __bootstrapRuntime() {
        if (hasBootstrapped) return;

        core.setMacrotaskCallback(timers.handleTimerMacrotask);

        delete window.console;
        delete window.Deno;
        delete window.__bootstrap;
        // https://github.com/denoland/deno/issues/4324
        delete Object.prototype.__proto__;

        hasBootstrapped = true
    }

    const globalScope = {
        atob: base64.atob,
        btoa: base64.btoa,

        /* setTimeout: timers.setTimeout,
        clearTimeout: timers.clearTimeout,
        setInterval: timers.setInterval,
        clearInterval: timers.clearInterval,
        sleep: timers.sleep, */
        print: text => core.opSync('op_print', text),

        Isolator: isolator,
        __bootstrapRuntime
    }

    ObjectAssign(window, globalScope)
})(globalThis)