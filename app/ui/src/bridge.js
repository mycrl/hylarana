
//if (!window.__ENABLE_BRIDGE_HOOK) {
//    window.__ENABLE_BRIDGE_HOOK = true;
//    
//    window.message.on(async (payload) => {
//        try {
//            if (!window.__BRIDGE_HOOK) {
//                throw new Error("not found bridge on hooks");
//            }
//            
//            const req = JSON.parse(payload);
//            const res = await window.__BRIDGE_HOOK(req);
//            callback(true, JSON.stringify(res));
//        } catch(e) {
//            callback(false, e.message);
//        }
//    });
//}
//
//export default class Bridge {
//    static On(handler) {
//        window.__BRIDGE_HOOK = handler;
//    }
//    
//    static Call(type, content) {
//        return new Promise((resolve, reject) => {
//            window.native.bridge.call(JSON.stringify({ type, content }), (err, res) => {
//                if (err) {
//                    reject(err)
//                } else {
//                    const it = JSON.parse(res);
//                    if (it.type == type) {
//                        resolve(it.content);
//                    }
//                }
//            });
//        });
//    }
//}