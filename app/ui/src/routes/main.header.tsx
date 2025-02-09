import "../styles/main.header.css";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faGear, faAngleLeft } from "@fortawesome/free-solid-svg-icons";
import { createLocalesStore } from "../locales";
import { useState } from "react";

export type Type = "settings" | "sender" | "receiver";

export default function ({
    defaultType,
    onChange,
}: {
    defaultType?: Type;
    onChange: (type: Type) => void;
}) {
    const Locales = createLocalesStore();
    const [type, setType] = useState(defaultType || "sender");

    function change(ty: Type) {
        setType(ty);
        onChange(ty);
    }

    return (
        <>
            <div id='Header'>
                <div id='navigation'>
                    <FontAwesomeIcon
                        className='icon click transition'
                        icon={type == "settings" ? faAngleLeft : faGear}
                        onClick={() => change(type == "settings" ? "sender" : "settings")}
                    />
                    <span>{type != "settings" ? Locales.Settings : Locales.BackToHome}</span>
                </div>
                {type != "settings" && (
                    <div id='switch'>
                        <div
                            className='item click transition left'
                            id={type == "sender" ? "selected" : ""}
                            onClick={() => change("sender")}
                        >
                            <p>{Locales.Sender}</p>
                        </div>
                        <div
                            className='item click transition right'
                            id={type == "receiver" ? "selected" : ""}
                            onClick={() => change("receiver")}
                        >
                            <p>{Locales.Receiver}</p>
                        </div>
                    </div>
                )}
            </div>
        </>
    );
}
