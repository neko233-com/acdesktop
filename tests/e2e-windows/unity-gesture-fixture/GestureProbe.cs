using System;
using System.IO;
using UnityEngine;

public sealed class GestureProbe : MonoBehaviour
{
    private string status = "Waiting for touch input";
    private string logPath;
    [RuntimeInitializeOnLoadMethod]
    private static void Create() { new GameObject("GestureProbe").AddComponent<GestureProbe>(); }
    private void Start() { Application.runInBackground = true; logPath = Path.Combine(Path.GetTempPath(), "acdesktop-unity-gesture-input.csv"); File.WriteAllText(logPath, "time,id,phase,x,y,count\n"); }
    private void Update()
    {
        foreach (Touch t in Input.touches)
        {
            status = "touch " + t.fingerId + " " + t.phase + " " + t.position + " count=" + Input.touchCount;
            File.AppendAllText(logPath, string.Format(System.Globalization.CultureInfo.InvariantCulture, "{0:F4},{1},{2},{3:F1},{4:F1},{5}\n", Time.realtimeSinceStartup, t.fingerId, t.phase, t.position.x, t.position.y, Input.touchCount));
        }
    }
    private void OnGUI() { GUI.Label(new Rect(20, 20, 900, 80), "Unity touch fixture: " + status + " frame=" + Time.frameCount); }
}
