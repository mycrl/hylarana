import "../styles/main.header.css";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faGear, faAngleLeft } from "@fortawesome/free-solid-svg-icons";
import { useState } from "react";
import { useAtomValue } from "jotai";
import { localesAtom } from "../state";

export type Type = "settings" | "sender" | "receiver";

export default function ({
    defaultType,
    onChange,
}: {
    defaultType?: Type;
    onChange: (type: Type) => void;
}) {
    const locales = useAtomValue(localesAtom);
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
                    <span>{type != "settings" ? locales.Settings : locales.BackToHome}</span>
                </div>
                {type != "settings" && (
                    <div id='switch'>
                        <div
                            className='item click transition left'
                            id={type == "sender" ? "selected" : ""}
                            onClick={() => change("sender")}
                        >
                            <p>{locales.Sender}</p>
                        </div>
                        <div
                            className='item click transition right'
                            id={type == "receiver" ? "selected" : ""}
                            onClick={() => change("receiver")}
                        >
                            <p>{locales.Receiver}</p>
                        </div>
                    </div>
                )}
            </div>
        </>
    );
}
