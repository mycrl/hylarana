import { useState } from "react";

type Ref<T> = {
    get: () => T;
    set: (value: T) => void;
};

export default function Input<T extends string | number | null>({
    ref,
    disabled,
}: {
    disabled: boolean;
    ref: Ref<T>;
}) {
    const [value, setValue] = useState(ref.get());
    const isNumber = typeof value == "number";

    return (
        <>
            <input
                type={isNumber ? "number" : "text"}
                value={value || ""}
                disabled={disabled}
                onChange={({ target }) => {
                    const value =
                        target.value.length == 0
                            ? null
                            : isNumber
                            ? Number(target.value)
                            : target.value;

                    setValue(value as T);
                    ref.set(value as T);
                }}
            />
        </>
    );
}
