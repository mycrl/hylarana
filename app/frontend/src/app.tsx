import "./styles/app.css";

import { useState } from "react";

import Header, { Type } from "./widgets/header";
import Receiver from "./widgets/receiver";
import Settings from "./widgets/settings";
import Sender from "./widgets/sender";
import Info from "./widgets/info";
import { cfg, TargetOs } from "./utils";

export default function () {
    const [type, setType] = useState<Type>("sender");

    return (
        <>
            <div
                id='app'
                style={{
                    fontSize: cfg(TargetOs.Android) ? "1rem" : "12px",
                }}
            >
                <Header value={type} onChange={setType} />
                <Sender
                    offscreen={type != "sender"}
                    style={{
                        left: type == "sender" ? "0" : "-100vw",
                    }}
                />
                <Receiver
                    offscreen={type != "receiver"}
                    style={{
                        left: type == "receiver" ? "0" : "100vw",
                    }}
                />
                <Settings
                    offscreen={type != "settings"}
                    style={{
                        top: type == "settings" ? "50px" : "110vh",
                    }}
                />
                <Info />
            </div>
        </>
    );
}
