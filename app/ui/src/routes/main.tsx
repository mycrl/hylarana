import { useState } from "react";
import MainHeader, { Type } from "./main.header";
import MainSender from "./main.sender";
import MainReceiver from "./main.receiver";
import MainInfo from "./main.info";
import Mainsettings from "./main.settings";
import "../styles/main.css";

export default function () {
    const [type, setType] = useState<Type>("sender");

    return (
        <>
            <div id='Main'>
                <MainHeader defaultType='sender' onChange={setType} />
                {type == "sender" && <MainSender />}
                {type == "receiver" && <MainReceiver />}
                {type == "settings" && <Mainsettings />}
                {type != "settings" && <MainInfo />}
            </div>
        </>
    );
}
