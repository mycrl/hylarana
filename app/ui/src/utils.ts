export function ONCE<T>(key: string, init: () => T): T {
    const name = "__" + key.toUpperCase();

    let global = window as any;
    if (!Object.hasOwn(global, name)) {
        global[name] = init();
    }

    return global[name];
}
