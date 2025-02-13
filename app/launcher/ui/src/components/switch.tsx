import { useState } from "react";
import "../styles/switch.css";

export interface Props {
    defaultValue: boolean;
    onChange?: (value: boolean) => void;
}

export default function ({ defaultValue, onChange }: Props) {
    const [value, setValue] = useState(defaultValue);

    function change() {
        setValue(!value);

        if (onChange) {
            onChange(!value);
        }
    }

    return (
        <>
            <div id='Switch' className='click' onClick={change}>
                <div className='round' id={value ? "selected" : ""}></div>
            </div>
        </>
    );
}
