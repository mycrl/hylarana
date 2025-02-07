export enum DeviceType {
    Windows = "Windows",
    Android = "Android",
    Apple = "Apple",
}

export interface DeviceInfo {
    addrs: string[];
    kind: DeviceType;
    name: string;
    port: number;
    description: string | null;
}
