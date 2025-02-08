import { useState } from "react";
import MainHeader, { Type } from "./main.header";
import MainSender from "./main.sender";
import MainReceiver from "./main.receiver";
import MainInfo from "./main.info";
import MainSettings from "./main.settings";
import "../styles/main.css";

export default function () {
    const [type, setType] = useState<Type>("sender");
    const [devices, setDevices] = useState([]);

    return (
        <>
            <div id='Main'>
                <MainHeader defaultType='sender' onChange={setType} />
                {type == "sender" && <MainSender devices={devices} />}
                {type == "receiver" && <MainReceiver devices={devices} />}
                {type == "settings" && <MainSettings />}
                {type != "settings" && <MainInfo />}
            </div>
        </>
    );
}
