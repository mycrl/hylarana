import "../styles/main.receiver.css";
import { useAtom, useAtomValue } from "jotai";
import { autoAllowAtom, localesAtom, statusAtom } from "../state";
import SenderImage from "../assets/sender.svg";
import ReceiverImage from "../assets/receiver.svg";
import Devices from "./main.receiver.devices";
import { closeReceiver } from "../core";
import { Status } from "@/hylarana";

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

export default function () {
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
            <div id='Receiver'>
                <div id='switch'>
                    <div className='body'>
                        <span>{locales.AutoAllow}</span>
                        <Switch
                            value={autoAllow}
                            onChange={(it) => {
                                setAutoAllow(it);
                            }}
                        />
                    </div>
                    <p>{locales.AutoAllowHelp}</p>
                </div>
                <div className='devices'>
                    {status == Status.Idle && <Devices />}
                    {status != Status.Idle && (
                        <div className='working'>
                            <img
                                src={status == Status.Receiving ? ReceiverImage : SenderImage}
                                style={{
                                    marginTop: status == Status.Receiving ? "100px" : "180px",
                                }}
                            />
                            <p>
                                {status == Status.Receiving
                                    ? locales.Receivering
                                    : locales.Sendering}
                            </p>
                            {status == Status.Receiving && <button onClick={stop}>停止</button>}
                        </div>
                    )}
                </div>
            </div>
        </>
    );
}
