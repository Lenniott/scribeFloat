import Foundation
import CoreAudio

private func propertyAddress(selector: AudioObjectPropertySelector) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress(
        mSelector: selector,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain
    )
}

private func deviceName(_ deviceID: AudioDeviceID) -> String? {
    var name: Unmanaged<CFString>?
    var size = UInt32(MemoryLayout<Unmanaged<CFString>?>.size)
    var addr = propertyAddress(selector: kAudioObjectPropertyName)
    let err = AudioObjectGetPropertyData(deviceID, &addr, 0, nil, &size, &name)
    guard err == noErr, let cf = name?.takeRetainedValue() else {
        return nil
    }
    return cf as String
}

private func outputDeviceIDs() -> [AudioDeviceID] {
    var addr = propertyAddress(selector: kAudioHardwarePropertyDevices)
    var dataSize: UInt32 = 0
    guard AudioObjectGetPropertyDataSize(
        AudioObjectID(kAudioObjectSystemObject),
        &addr,
        0,
        nil,
        &dataSize
    ) == noErr else {
        return []
    }
    let count = Int(dataSize) / MemoryLayout<AudioDeviceID>.size
    var ids = [AudioDeviceID](repeating: 0, count: count)
    guard AudioObjectGetPropertyData(
        AudioObjectID(kAudioObjectSystemObject),
        &addr,
        0,
        nil,
        &dataSize,
        &ids
    ) == noErr else {
        return []
    }
    return ids.filter(isOutputDevice)
}

private func isOutputDevice(_ id: AudioDeviceID) -> Bool {
    var streamCount: UInt32 = 0
    var size = UInt32(MemoryLayout<UInt32>.size)
    var addr = AudioObjectPropertyAddress(
        mSelector: kAudioDevicePropertyStreams,
        mScope: kAudioDevicePropertyScopeOutput,
        mElement: kAudioObjectPropertyElementMain
    )
    let err = AudioObjectGetPropertyData(id, &addr, 0, nil, &size, &streamCount)
    return err == noErr && streamCount > 0
}

private func defaultOutputDeviceID() -> AudioDeviceID? {
    var deviceID = AudioDeviceID(0)
    var size = UInt32(MemoryLayout<AudioDeviceID>.size)
    var addr = propertyAddress(selector: kAudioHardwarePropertyDefaultOutputDevice)
    let err = AudioObjectGetPropertyData(
        AudioObjectID(kAudioObjectSystemObject),
        &addr,
        0,
        nil,
        &size,
        &deviceID
    )
    guard err == noErr, deviceID != kAudioObjectUnknown else {
        return nil
    }
    return deviceID
}

private func cmdGetDefaultOutput() -> Int32 {
    guard let id = defaultOutputDeviceID(), let name = deviceName(id) else {
        fputs("failed to read default output device\n", stderr)
        return 1
    }
    print(name)
    return 0
}

private func cmdSetDefaultOutput(_ targetName: String) -> Int32 {
    for id in outputDeviceIDs() {
        guard let name = deviceName(id), name == targetName else {
            continue
        }
        var deviceID = id
        var size = UInt32(MemoryLayout<AudioDeviceID>.size)
        var addr = propertyAddress(selector: kAudioHardwarePropertyDefaultOutputDevice)
        let err = AudioObjectSetPropertyData(
            AudioObjectID(kAudioObjectSystemObject),
            &addr,
            0,
            nil,
            size,
            &deviceID
        )
        if err == noErr {
            return 0
        }
        fputs("failed to set default output device (OSStatus \(err))\n", stderr)
        return 1
    }
    fputs("output device not found: \(targetName)\n", stderr)
    return 1
}

let args = CommandLine.arguments
if args.count < 2 {
    fputs("usage: set-default-output get-default-output|set-default-output <name>\n", stderr)
    exit(2)
}

switch args[1] {
case "get-default-output":
    exit(cmdGetDefaultOutput())
case "set-default-output":
    guard args.count >= 3 else {
        fputs("usage: set-default-output set-default-output <device name>\n", stderr)
        exit(2)
    }
    exit(cmdSetDefaultOutput(args[2]))
default:
    fputs("unknown command: \(args[1])\n", stderr)
    exit(2)
}
