import type { SettingsRef } from "./settings";
import { Language, LanguageOptions } from "../state";

import Input from "../components/input";
import Select from "../components/select";

export default function ({
    settings,
    locales,
    disabled,
}: {
    settings: SettingsRef;
    locales: Language;
    disabled: boolean;
}) {
    return (
        <>
            <div className='module'>
                <h1>{locales.System}</h1>
                <div className='item'>
                    <p>{locales.DeviceName}:</p>
                    <Input ref={settings.system.name} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.Language}:</p>
                    <Select
                        ref={settings.system.language as any}
                        options={LanguageOptions}
                        disabled={disabled}
                    />
                </div>
            </div>
        </>
    );
}
