import "./style.css";

export interface Props {
    value: boolean;
    onChange: () => void;
}

export default function ({ value, onChange }: Props) {
    return (
        <>
            <div id='Switch' className='click' onClick={onChange}>
                <div className='round' id={value ? "selected" : ""}></div>
            </div>
        </>
    );
}
