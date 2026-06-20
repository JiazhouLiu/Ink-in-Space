using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Net;
using System.Net.Sockets;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;

public class MxInkHandler : StylusHandler
{
    private readonly UdpClient _socket = new();

    public Color active_color = Color.gray;
    public Color double_tap_active_color = Color.cyan;
    public Color default_color = Color.black;

    [SerializeField]
    private InputActionReference _tipActionRef;
    [SerializeField]
    private InputActionReference _grabActionRef;
    [SerializeField]
    private InputActionReference _optionActionRef;
    [SerializeField]
    private InputActionReference _middleActionRef;
    private float _hapticClickDuration = 0.011f;
    private float _hapticClickAmplitude = 1.0f;
    [SerializeField] private GameObject _tip;
    [SerializeField] private GameObject _cluster_front;
    [SerializeField] private GameObject _cluster_middle;
    [SerializeField] private GameObject _cluster_back;

    private void Awake()
    {
        _tipActionRef.action.Enable();
        _grabActionRef.action.Enable();
        _optionActionRef.action.Enable();
        _middleActionRef.action.Enable();

        InputSystem.onDeviceChange += OnDeviceChange;
    }

    private void OnDeviceChange(InputDevice device, InputDeviceChange change)
    {
        if (device.name.ToLower().Contains("logitech"))
        {
            switch (change)
            {
                case InputDeviceChange.Disconnected:
                    _tipActionRef.action.Disable();
                    _grabActionRef.action.Disable();
                    _optionActionRef.action.Disable();
                    _middleActionRef.action.Disable();
                    break;
                case InputDeviceChange.Reconnected:
                    _tipActionRef.action.Enable();
                    _grabActionRef.action.Enable();
                    _optionActionRef.action.Enable();
                    _middleActionRef.action.Enable();
                    break;
            }
        }
    }

    void Update()
    {
        _stylus.inkingPose.position = transform.position;
        _stylus.inkingPose.rotation = transform.rotation;
        _stylus.tip_value = _tipActionRef.action.ReadValue<float>();
        _stylus.cluster_middle_value = _middleActionRef.action.ReadValue<float>();
        _stylus.cluster_front_value = _grabActionRef.action.IsPressed();
        _stylus.cluster_back_value = _optionActionRef.action.IsPressed();

        byte buttons = 0;
        if (_stylus.tip_value > 0)
        {
            buttons |= (byte)(Math.Max((byte)(_stylus.tip_value * 7), (byte)1) << 5);
        }
        if (_stylus.cluster_front_value)
        {
            buttons |= 0b00010000;
        }
        if (_stylus.cluster_middle_value > 0)
        {
            buttons |= (byte)(Math.Min(Math.Max((byte)(_stylus.cluster_middle_value * 7), (byte)1), (byte)7) << 1);
        }
        if (_stylus.cluster_back_value)
        {
            buttons |= 0b00000001;
        }

        byte[] message = BitConverter.GetBytes(_stylus.inkingPose.position.x)
            .Concat(BitConverter.GetBytes(_stylus.inkingPose.position.y))
            .Concat(BitConverter.GetBytes(_stylus.inkingPose.position.z))
            .Concat(BitConverter.GetBytes(_stylus.inkingPose.rotation.x))
            .Concat(BitConverter.GetBytes(_stylus.inkingPose.rotation.y))
            .Concat(BitConverter.GetBytes(_stylus.inkingPose.rotation.z))
            .Concat(BitConverter.GetBytes(_stylus.inkingPose.rotation.w))
            .Concat(new byte[] { buttons })
            .ToArray();
        _socket.Send(message, message.Length, "127.0.0.1", 1273);

        _tip.GetComponent<MeshRenderer>().material.color = _stylus.tip_value > 0 ? active_color : default_color;
        _cluster_front.GetComponent<MeshRenderer>().material.color = _stylus.cluster_front_value ? active_color : default_color;
        _cluster_middle.GetComponent<MeshRenderer>().material.color = _stylus.cluster_middle_value > 0 ? active_color : default_color;
        _cluster_back.GetComponent<MeshRenderer>().material.color = _stylus.cluster_back_value ? active_color : default_color;
    }

    public void TriggerHapticPulse(float amplitude, float duration)
    {
        var device = UnityEngine.XR.InputDevices.GetDeviceAtXRNode(_stylus.isOnRightHand ? UnityEngine.XR.XRNode.RightHand : UnityEngine.XR.XRNode.LeftHand);
        device.SendHapticImpulse(0, amplitude, duration);
    }

    public void TriggerHapticClick()
    {
        TriggerHapticPulse(_hapticClickAmplitude, _hapticClickDuration);
    }

    public override bool CanDraw()
    {
        return true;
    }
}
