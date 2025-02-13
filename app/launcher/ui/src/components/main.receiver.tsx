import "../styles/main.receiver.css";
import Devices from "./main.receiver.devices";
import Switch from "./switch";
import { useAtomValue } from "jotai";
import { localesAtom, statusAtom } from "../state";
import SenderImage from "../assets/sender.svg";
import ReceiverImage from "../assets/receiver.svg";
import { closeReceiver, Status } from "../hylarana";

export default function () {
    const locales = useAtomValue(localesAtom);
    const status = useAtomValue(statusAtom);

    async function stop() {
        await closeReceiver();
    }

    return (
        <>
            <div id='Receiver'>
                <div id='switch'>
                    <div className='body'>
                        <span>{locales.AutoAllow}</span>
                        <Switch defaultValue={false} />
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
