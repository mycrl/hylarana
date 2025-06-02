import "../styles/receiver.css";

import { useAtom, useAtomValue } from "jotai";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faPowerOff, faCircleInfo } from "@fortawesome/free-solid-svg-icons";

import Devices from "./receiver.devices";
import SenderImage from "../assets/sender.svg";
import ReceiverImage from "../assets/receiver.svg";

import { closeReceiver } from "../core";
import { autoAllowAtom, localesAtom, statusAtom } from "../state";
import { Status } from "../bridge";

function Switch({ value, onChange }: { value: boolean; onChange?: (value: boolean) => void }) {
    return (
        <>
            <div
                id='Switch'
                className='click'
                onClick={() => {
                    if (onChange) {
                        onChange(!value);
                    }
                }}
            >
                <div className='round' id={value ? "selected" : ""}></div>
            </div>
        </>
    );
}

export default function ({
    offscreen,
    style,
}: {
    offscreen: boolean;
    style?: React.CSSProperties;
}) {
    const [autoAllow, setAutoAllow] = useAtom(autoAllowAtom);
    const locales = useAtomValue(localesAtom);
    const status = useAtomValue(statusAtom);

    async function stop() {
        if (status == Status.Receiving) {
            await closeReceiver();
        }
    }

    return (
        <>
            <div className='Receiver' style={style}>
                {status == Status.Idle && (
                    <div className='Toggle'>
                        <div className='body'>
                            <span>{locales.AutoAllow}</span>
                            <Switch
                                value={autoAllow}
                                onChange={(it) => {
                                    setAutoAllow(it);
                                }}
                            />
                        </div>
                        <p>
                            <FontAwesomeIcon icon={faCircleInfo} />
                            {" " + locales.AutoAllowHelp}
                        </p>
                    </div>
                )}
                <div className='Content'>
                    {status == Status.Idle && <Devices offscreen={offscreen} />}
                    {status != Status.Idle && (
                        <div className='Working'>
                            <img
                                src={status == Status.Receiving ? ReceiverImage : SenderImage}
                                style={{
                                    marginTop:
                                        status == Status.Receiving
                                            ? "calc(50vh - 200px)"
                                            : "calc(50vh - 130px)",
                                }}
                            />
                            <p>
                                {status == Status.Receiving
                                    ? locales.Receivering
                                    : locales.Sendering}
                            </p>
                            {status == Status.Receiving && (
                                <button className='click' onClick={stop}>
                                    <FontAwesomeIcon icon={faPowerOff} />
                                    <span>{locales.ReceiverStop}</span>
                                </button>
                            )}
                        </div>
                    )}
                </div>
            </div>
        </>
    );
}
