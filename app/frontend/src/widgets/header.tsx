import "../styles/header.css";

import { useRef } from "react";
import { useAtomValue } from "jotai";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faGear, faAngleLeft } from "@fortawesome/free-solid-svg-icons";

import { localesAtom } from "../state";

export type Type = "settings" | "sender" | "receiver";

export default function ({ value, onChange }: { value: Type; onChange: (type: Type) => void }) {
    const locales = useAtomValue(localesAtom);
    const toggleRef = useRef<HTMLDivElement | null>(null);

    return (
        <>
            <div className='Header'>
                <div className='Navigation'>
                    <FontAwesomeIcon
                        className='icon click'
                        icon={value == "settings" ? faAngleLeft : faGear}
                        onClick={() => onChange(value == "settings" ? "sender" : "settings")}
                    />
                    <span>{value != "settings" ? locales.Settings : locales.BackToHome}</span>
                </div>
                <div
                    ref={toggleRef}
                    className='Toggle'
                    style={{
                        top: value == "settings" ? "-100px" : "0",
                    }}
                >
                    <div
                        id='slider'
                        style={{
                            left: value == "sender" ? 0 : "106px",
                        }}
                    ></div>
                    <div className='item click' onClick={() => onChange("sender")}>
                        <p
                            style={{
                                color:
                                    value == "sender" ? undefined : "var(--secondary-text-color)",
                            }}
                        >
                            {locales.Sender}
                        </p>
                    </div>
                    <div className='item click' onClick={() => onChange("receiver")}>
                        <p
                            style={{
                                color:
                                    value == "receiver" ? undefined : "var(--secondary-text-color)",
                            }}
                        >
                            {locales.Receiver}
                        </p>
                    </div>
                </div>
            </div>
        </>
    );
}
