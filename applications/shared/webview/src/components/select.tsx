import { useState } from "react";

type Ref<T> = {
    get: () => T;
    set: (value: T) => void;
};

export default function Select<T extends string | number>({
    ref,
    options,
    disabled,
    onChange,
}: {
    disabled: boolean;
    ref: Ref<T>;
    options: { [k: string]: T };
    onChange?: (v: T) => void;
}) {
    const [value, setValue] = useState(ref.get());
    const isNumber = typeof value == "number";

    return (
        <>
            <select
                value={value}
                disabled={disabled}
                onChange={({ target }) => {
                    const value = isNumber ? Number(target.value) : target.value;

                    setValue(target.value as any);
                    ref.set(value as T);

                    if (onChange) {
                        onChange(value as T);
                    }
                }}
            >
                {Object.entries(options).map(([k, v]) => (
                    <option key={k} value={k}>
                        {v}
                    </option>
                ))}
            </select>
        </>
    );
}
