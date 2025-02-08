class Events {
    private listeners: { [k: string]: { [k: string]: (args: any) => void } } = {};
    private sequence: number = 0;

    on(event: string, handle: (args: any) => void): number {
        const sequence = this.sequence;
        if (this.sequence >= 65535) {
            this.sequence = 0;
        } else {
            this.sequence += 1;
        }

        if (!this.listeners[event]) {
            this.listeners[event] = {};
        }

        this.listeners[event][sequence] = handle;
        return sequence;
    }

    emit(event: string, args?: any) {
        if (this.listeners[event]) {
            for (const handle of Object.values(this.listeners[event])) {
                handle(args);
            }
        }
    }

    remove(sequence: number) {
        let seq = String(sequence);
        for (const [key, listener] of Object.entries(this.listeners)) {
            for (const it of Object.keys(listener)) {
                if (it == seq) {
                    delete this.listeners[key][it];
                    return;
                }
            }
        }
    }
}

declare global {
    interface Window {
        __EVENTS?: Events;
    }
}

if (!window.__EVENTS) {
    window.__EVENTS = new Events();
}

export default window.__EVENTS!;
