using SaveForge.NewPlugin;

// ── Locate solution root ──────────────────────────────────────────────────────
var solutionRoot = FindSolutionRoot(Directory.GetCurrentDirectory())
    ?? throw new InvalidOperationException(
        "Could not find SaveForge.sln. Run this tool from within the SaveForge repository.");

// ── Parse command-line args ───────────────────────────────────────────────────
var args = ParseArgs(Environment.GetCommandLineArgs()[1..]);

// ── Collect options (args → prompt for missing) ───────────────────────────────
Console.WriteLine("SaveForge — New Plugin Scaffolder");
Console.WriteLine("──────────────────────────────────");

var name        = Require(args, "--name",          "Plugin name (PascalCase, no spaces, e.g. MyGame):", ValidateName);
var displayName = Optional(args, "--display-name") ?? $"{SplitCamel(name)} Save Editor";
var author      = Require(args, "--author",         "Author name:");
var description = Require(args, "--description",    "Short description:");
var extensions  = ParseExtensions(
    Optional(args, "--extensions") ?? Prompt("File extensions (comma-separated, e.g. .sav,.json):", ".json"));
var detectionKey = Optional(args, "--detection-key")
    ?? Prompt("Detection key — unique string in the file header (leave blank to add manually):", "");
var saveDir      = Optional(args, "--save-dir")
    ?? Prompt("Default save directory (may use %APPDATA%, leave blank if unknown):", "");
var editorType   = ParseEditorType(
    Optional(args, "--type") ?? PromptChoice("Editor type:", ["schema", "custom"], "schema"));

var options = new PluginOptions
{
    Name         = name,
    DisplayName  = displayName,
    Author       = author,
    Description  = description,
    Extensions   = extensions,
    DetectionKey = detectionKey,
    SaveDirectory = saveDir,
    EditorType   = editorType,
};

// ── Confirm ───────────────────────────────────────────────────────────────────
Console.WriteLine();
Console.WriteLine("Summary:");
Console.WriteLine($"  Name:           {options.Name}");
Console.WriteLine($"  Display name:   {options.DisplayName}");
Console.WriteLine($"  Plugin ID:      {options.Id}");
Console.WriteLine($"  Assembly:       {options.Assembly}");
Console.WriteLine($"  Author:         {options.Author}");
Console.WriteLine($"  Description:    {options.Description}");
Console.WriteLine($"  Extensions:     {string.Join(", ", options.Extensions)}");
Console.WriteLine($"  Detection key:  {(string.IsNullOrWhiteSpace(options.DetectionKey) ? "(none — add manually)" : options.DetectionKey)}");
Console.WriteLine($"  Save directory: {(string.IsNullOrWhiteSpace(options.SaveDirectory) ? "(none)" : options.SaveDirectory)}");
Console.WriteLine($"  Editor type:    {options.EditorType}");
Console.WriteLine();
Console.Write("Proceed? [Y/n] ");
var proceed = Console.ReadLine()?.Trim().ToLowerInvariant();
if (proceed is "n" or "no")
{
    Console.WriteLine("Aborted.");
    return 0;
}

// ── Scaffold ──────────────────────────────────────────────────────────────────
new PluginScaffolder(solutionRoot).Run(options);
return 0;

// ── Helpers ───────────────────────────────────────────────────────────────────

static string? FindSolutionRoot(string start)
{
    var dir = new DirectoryInfo(start);
    while (dir is not null)
    {
        if (dir.GetFiles("SaveForge.sln").Length > 0)
            return dir.FullName;
        dir = dir.Parent;
    }
    return null;
}

static Dictionary<string, string> ParseArgs(string[] argv)
{
    var result = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
    for (int i = 0; i < argv.Length - 1; i++)
        if (argv[i].StartsWith("--", StringComparison.Ordinal))
            result[argv[i]] = argv[i + 1];
    return result;
}

static string? Optional(Dictionary<string, string> args, string key) =>
    args.TryGetValue(key, out var v) ? v : null;

static string Require(Dictionary<string, string> args, string key, string prompt,
    Func<string, string?>? validate = null)
{
    var value = Optional(args, key);
    if (!string.IsNullOrWhiteSpace(value))
    {
        var err = validate?.Invoke(value);
        if (err is not null)
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.Error.WriteLine($"Invalid value for {key}: {err}");
            Console.ResetColor();
            Environment.Exit(1);
        }
        return value;
    }
    return Prompt(prompt, null, validate);
}

static string Prompt(string prompt, string? defaultValue = null, Func<string, string?>? validate = null)
{
    while (true)
    {
        Console.Write(defaultValue is not null ? $"{prompt} [{defaultValue}] " : $"{prompt} ");
        var input = Console.ReadLine()?.Trim() ?? "";
        if (string.IsNullOrEmpty(input) && defaultValue is not null)
            input = defaultValue;
        if (string.IsNullOrEmpty(input) && defaultValue is null)
            continue;

        var err = validate?.Invoke(input);
        if (err is not null)
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"  {err}");
            Console.ResetColor();
            continue;
        }
        return input;
    }
}

static string PromptChoice(string prompt, string[] choices, string defaultValue)
{
    Console.Write($"{prompt} ({string.Join("/", choices)}) [{defaultValue}] ");
    while (true)
    {
        var input = Console.ReadLine()?.Trim().ToLowerInvariant() ?? "";
        if (string.IsNullOrEmpty(input)) return defaultValue;
        if (Array.Exists(choices, c => c.Equals(input, StringComparison.OrdinalIgnoreCase)))
            return input;
        Console.Write($"  Choose one of: {string.Join(", ", choices)}: ");
    }
}

static string? ValidateName(string v)
{
    if (string.IsNullOrWhiteSpace(v)) return "Name cannot be empty.";
    if (v.Any(char.IsWhiteSpace))    return "Name must not contain spaces (use PascalCase, e.g. MyGame).";
    if (!char.IsUpper(v[0]))         return "Name must start with an uppercase letter (e.g. MyGame).";
    if (v.Any(c => !char.IsLetterOrDigit(c))) return "Name must contain only letters and digits.";
    return null;
}

static string[] ParseExtensions(string raw) =>
    raw.Split([',', ' '], StringSplitOptions.RemoveEmptyEntries)
       .Select(e => e.StartsWith('.') ? e : $".{e}")
       .ToArray();

static EditorType ParseEditorType(string raw) =>
    raw.Equals("custom", StringComparison.OrdinalIgnoreCase)
        ? EditorType.Custom
        : EditorType.Schema;

static string SplitCamel(string pascal)
{
    var sb = new System.Text.StringBuilder();
    for (int i = 0; i < pascal.Length; i++)
    {
        if (i > 0 && char.IsUpper(pascal[i]))
            sb.Append(' ');
        sb.Append(pascal[i]);
    }
    return sb.ToString();
}
