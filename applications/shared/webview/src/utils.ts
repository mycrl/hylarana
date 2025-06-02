export enum TargetOs {
    Windows,
    Macos,
    Linux,
    Android,
}

export const TARGET_OS = /Macintosh/i.test(navigator.userAgent)
    ? TargetOs.Macos
    : /Android/i.test(navigator.userAgent)
    ? TargetOs.Android
    : /Win/i.test(navigator.userAgent)
    ? TargetOs.Windows
    : TargetOs.Linux;

export function cfg(target: TargetOs): boolean {
    return target == TARGET_OS;
}
