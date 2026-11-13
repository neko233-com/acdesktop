using System;
using System.Globalization;
using System.IO;
using UnityEngine;

public sealed class GameplayArena : MonoBehaviour
{
    private string logPath;
    private int joystickFinger = -1;
    private Vector2 origin;
    private bool moving;
    private int score;
    private float previousSuccess = -1f;
    private string status = "Move the left joystick and tap the right button";

    [RuntimeInitializeOnLoadMethod]
    private static void Create() { new GameObject("GameplayArena").AddComponent<GameplayArena>(); }

    private void Start()
    {
        logPath = Path.Combine(Path.GetTempPath(), "acdesktop-unity-gameplay.csv");
        File.WriteAllText(logPath, "time,event,score,interval\n");
    }

    private void Update()
    {
        foreach (Touch touch in Input.touches)
        {
            if (touch.phase == TouchPhase.Began && joystickFinger < 0 && touch.position.x < Screen.width * 0.4f)
            {
                joystickFinger = touch.fingerId;
                origin = touch.position;
                moving = false;
                Record("joystick_down", -1f);
            }
            else if (touch.fingerId == joystickFinger && touch.phase == TouchPhase.Moved && touch.position.x - origin.x >= 30f && !moving)
            {
                moving = true;
                Record("joystick_moved", -1f);
            }
            else if (touch.phase == TouchPhase.Began && touch.fingerId != joystickFinger && touch.position.x > Screen.width * 0.6f)
            {
                if (moving)
                {
                    float now = Time.realtimeSinceStartup;
                    float interval = previousSuccess < 0 ? -1f : now - previousSuccess;
                    previousSuccess = now;
                    score++;
                    status = "Success " + score;
                    Record("success", interval);
                }
                else
                {
                    status = "Move the joystick before firing";
                    Record("early_fire", -1f);
                }
            }
            if (touch.fingerId == joystickFinger && (touch.phase == TouchPhase.Ended || touch.phase == TouchPhase.Canceled))
            {
                joystickFinger = -1;
                moving = false;
                Record("joystick_up", -1f);
            }
        }
    }

    private void Record(string kind, float interval)
    {
        File.AppendAllText(logPath, string.Format(CultureInfo.InvariantCulture, "{0:F4},{1},{2},{3:F4}\n", Time.realtimeSinceStartup, kind, score, interval));
    }

    private void OnGUI()
    {
        GUI.Box(new Rect(10, 90, Screen.width * 0.4f - 20, Screen.height - 100), "JOYSTICK ZONE");
        GUI.Box(new Rect(Screen.width * 0.6f, 90, Screen.width * 0.4f - 10, Screen.height - 100), "FIRE ZONE");
        GUI.Box(new Rect(Screen.width * (moving ? 0.48f : 0.43f), Screen.height * 0.5f, 60, 40), "PLAYER");
        GUI.Label(new Rect(20, 50, 700, 40), status);
    }
}
