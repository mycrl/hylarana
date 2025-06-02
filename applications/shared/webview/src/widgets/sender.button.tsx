import "../styles/sender.button.css";

import { useState } from "react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faPodcast, faPowerOff } from "@fortawesome/free-solid-svg-icons";

import { Language } from "../state";
import { Status } from "../bridge";

export default function ({
    status,
    locales,
    onStart,
    onStop,
}: {
    disabled?: boolean;
    status: Status;
    locales: Language;
    onStart: () => Promise<void>;
    onStop: () => Promise<void>;
}) {
    const [loading, setLoading] = useState(false);

    return (
        <>
            <button
                className={[
                    status == Status.Idle
                        ? loading
                            ? "ButtonLoading"
                            : "Button"
                        : "ButtonComplete",
                    "click",
                ].join(" ")}
                onClick={async () => {
                    if (!loading) {
                        setLoading(true);

                        try {
                            if (status == Status.Idle) {
                                await onStart();
                            } else {
                                await onStop();
                            }
                        } catch (e) {
                            console.error(e);
                        }

                        setLoading(false);
                    }
                }}
            >
                <FontAwesomeIcon
                    className='icon'
                    icon={status == Status.Sending ? faPowerOff : faPodcast}
                />
                <span>{status == Status.Sending ? locales.SenderStop : locales.SenderStart}</span>
            </button>
        </>
    );
}
