using System.Diagnostics;

namespace SaveForge.NewPlugin;

internal sealed class PluginScaffolder(string solutionRoot)
{
    public void Run(PluginOptions o)
    {
        var projectDir = Path.Combine(solutionRoot, o.RelativeProjectDir);

        Console.WriteLine();
        Console.WriteLine($"Scaffolding plugin at: {projectDir}");
        Console.WriteLine();

        if (Directory.Exists(projectDir))
        {
            Console.ForegroundColor = ConsoleColor.Yellow;
            Console.WriteLine($"Warning: directory already exists: {projectDir}");
            Console.ResetColor();
            Console.Write("Continue and overwrite? [y/N] ");
            var confirm = Console.ReadLine()?.Trim().ToLowerInvariant();
            if (confirm != "y")
            {
                Console.WriteLine("Aborted.");
                return;
            }
        }

        // ── Create directories ────────────────────────────────────────────────
        Directory.CreateDirectory(projectDir);
        Directory.CreateDirectory(Path.Combine(projectDir, "Models"));

        if (o.EditorType == EditorType.Custom)
        {
            Directory.CreateDirectory(Path.Combine(projectDir, "Views"));
            Directory.CreateDirectory(Path.Combine(projectDir, "ViewModels"));
        }

        // ── Write files ───────────────────────────────────────────────────────
        WriteFile(projectDir, $"{o.Assembly}.csproj",        FileTemplates.Csproj(o));
        WriteFile(projectDir, "manifest.json",                FileTemplates.Manifest(o));
        WriteFile(projectDir, $"{o.Name}Plugin.cs",           FileTemplates.PluginClass(o));
        WriteFile(projectDir, $"{o.Name}Document.cs",         FileTemplates.DocumentClass(o));

        if (o.EditorType == EditorType.Schema)
        {
            WriteFile(projectDir, $"{o.Name}Schema.cs", FileTemplates.SchemaClass(o));
        }
        else
        {
            WriteFile(projectDir, $"{o.Name}EditorViewFactory.cs",
                FileTemplates.ViewFactory(o));
            WriteFile(Path.Combine(projectDir, "ViewModels"), $"{o.Name}EditorViewModel.cs",
                FileTemplates.EditorViewModel(o));
            WriteFile(Path.Combine(projectDir, "Views"), $"{o.Name}EditorView.axaml",
                FileTemplates.EditorViewAxaml(o));
            WriteFile(Path.Combine(projectDir, "Views"), $"{o.Name}EditorView.axaml.cs",
                FileTemplates.EditorViewCodeBehind(o));
        }

        // ── Register with solution ────────────────────────────────────────────
        var slnFile = Path.Combine(solutionRoot, "SaveForge.sln");
        var csprojRelative = Path.Combine(o.RelativeProjectDir, $"{o.Assembly}.csproj");

        Console.WriteLine();
        Console.Write("Adding project to solution... ");
        var (success, output) = RunDotnet(solutionRoot, $"sln \"{slnFile}\" add \"{csprojRelative}\"");
        if (success)
        {
            Console.ForegroundColor = ConsoleColor.Green;
            Console.WriteLine("done.");
            Console.ResetColor();
        }
        else
        {
            Console.ForegroundColor = ConsoleColor.Yellow;
            Console.WriteLine("failed (add manually with the command below).");
            Console.WriteLine($"  dotnet sln add \"{csprojRelative}\"");
            Console.ResetColor();
            if (!string.IsNullOrWhiteSpace(output))
                Console.WriteLine(output);
        }

        // ── Summary ───────────────────────────────────────────────────────────
        Console.WriteLine();
        Console.ForegroundColor = ConsoleColor.Green;
        Console.WriteLine("Plugin scaffold created successfully!");
        Console.ResetColor();
        Console.WriteLine();
        Console.WriteLine("Files written:");
        foreach (var f in Directory.GetFiles(projectDir, "*", SearchOption.AllDirectories))
            Console.WriteLine($"  {Path.GetRelativePath(solutionRoot, f)}");

        Console.WriteLine();
        Console.WriteLine("Next steps:");
        Console.WriteLine($"  1. Implement {o.Name}Document — parse the save file format");
        Console.WriteLine($"  2. Implement GetValue/SetValue — map dot-paths to your model");
        if (o.EditorType == EditorType.Schema)
            Console.WriteLine($"  3. Fill in {o.Name}Schema — declare what fields to show");
        else
            Console.WriteLine($"  3. Build out {o.Name}EditorView.axaml — design your custom UI");
        Console.WriteLine($"  4. Run: dotnet build src/{o.Assembly}/");
        Console.WriteLine($"  5. Run: dotnet run --project src/SaveForge.App/");
        Console.WriteLine();
        Console.WriteLine($"See docs/PLUGIN_AUTHORING.md for the full guide.");
    }

    private static void WriteFile(string dir, string fileName, string content)
    {
        var path = Path.Combine(dir, fileName);
        File.WriteAllText(path, content);
        Console.WriteLine($"  created  {fileName}");
    }

    private static (bool Success, string Output) RunDotnet(string workingDir, string args)
    {
        var psi = new ProcessStartInfo("dotnet", args)
        {
            WorkingDirectory   = workingDir,
            RedirectStandardOutput = true,
            RedirectStandardError  = true,
            UseShellExecute    = false,
        };

        using var proc = Process.Start(psi);
        if (proc is null) return (false, "");

        var stdout = proc.StandardOutput.ReadToEnd();
        var stderr = proc.StandardError.ReadToEnd();
        proc.WaitForExit();

        return (proc.ExitCode == 0, stdout + stderr);
    }
}
