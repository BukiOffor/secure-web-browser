#Requires AutoHotkey v2.0  ; or v1.1
#Include Interception.ahk

; Define the keys to block (Ctrl, Alt, and Delete)
Ctrl := 0x11
Alt  := 0x12
Delete := 0x2E

; Create an Interception context
ctx := Interception.CreateContext()

; Start intercepting keyboard input
Interception.SetFilter(ctx, "keyboard")

; Define the callback function to handle keyboard events
OnInterceptionCallback(ctx, deviceType, keyStroke, keystrokeState) {
    if (deviceType = "keyboard") {
        if (keystrokeState = "down") {
            ; Check if Ctrl, Alt, and Delete are pressed simultaneously
            if (keyStroke.Key == Ctrl && GetKeyState("Alt", "P") && keyStroke.Key == Delete) {
                ; Prevent the keystroke from being processed
                return false ; Prevents the keystroke from being sent to the system
            }
        }
    }
    ; Allow other keys to pass through
    return true ; Allow the keystroke to pass through if it's not the blocked combination
}

; Set the callback function
Interception.SetCallback(ctx, "OnInterceptionCallback")

; Keep the script running
Return

; Example of how to disable the interception (use with care!)
!F8::  ;Example hotkey, press Alt+F8 to stop intercepting
    Interception.RemoveCallback(ctx)
    Interception.DestroyContext(ctx)
    Suspend Off
Return
