using System.IO;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine.SceneManagement;
using UnityEngine;
using UnityEngine.Rendering;

public static class GestureBuilder
{
    public static void Build()
    {
        GraphicsSettings.defaultRenderPipeline = null;
        QualitySettings.renderPipeline = null;
        var scene = EditorSceneManager.NewScene(NewSceneSetup.DefaultGameObjects, NewSceneMode.Single);
        EditorSceneManager.SaveScene(scene, "Assets/GestureLab.unity");
        var output = Path.Combine(Path.GetTempPath(), "acdesktop-gesture-build", "GestureLab.exe");
        Directory.CreateDirectory(Path.GetDirectoryName(output));
        var result = BuildPipeline.BuildPlayer(new[] { "Assets/GestureLab.unity" }, output, BuildTarget.StandaloneWindows64, BuildOptions.Development);
        if (result.summary.result != UnityEditor.Build.Reporting.BuildResult.Succeeded) throw new System.Exception("Unity fixture build failed: " + result.summary.result);
    }
}

