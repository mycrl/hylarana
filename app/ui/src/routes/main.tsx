import "../styles/main.css";
import { useState } from "react";
import MainHeader, { Type } from "../components/main.header";
import MainReceiver from "../components/main.receiver";
import Mainsettings from "../components/main.settings";
import MainSender from "../components/main.sender";
import MainInfo from "../components/main.info";

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
