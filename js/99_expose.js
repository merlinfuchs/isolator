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

    const console = new bootstrap.console.Console(text => isolator.makeResourceRequest('console', text));

    function __bootstrapRuntime() {
        if (hasBootstrapped) return;

        core.setMacrotaskCallback(timers.handleTimerMacrotask);

        delete window.Deno;
        delete window.__bootstrap;
        // https://github.com/denoland/deno/issues/4324
        delete Object.prototype.__proto__;

        hasBootstrapped = true
    }

    const globalScope = {
        atob: base64.atob,
        btoa: base64.btoa,

        setTimeout: timers.setTimeout,
        clearTimeout: timers.clearTimeout,
        setInterval: timers.setInterval,
        clearInterval: timers.clearInterval,
        sleep: timers.sleep,
        console,

        Isolator: isolator,
        __bootstrapRuntime
    }

    ObjectAssign(window, globalScope)
})(globalThis)