namespace VoiceInsert.App.Models;

public sealed partial class ApiProfile : CommunityToolkit.Mvvm.ComponentModel.ObservableObject
{
    public string Id { get; set; } = Guid.NewGuid().ToString("N");
    [CommunityToolkit.Mvvm.ComponentModel.ObservableProperty]
    private string _name = "wlast";
    public string BaseUrl { get; set; } = "http://127.0.0.1:9573";
    public string Model { get; set; } = "";
    public string TranslationModel { get; set; } = "";
    public string Language { get; set; } = "";
    public string Prompt { get; set; } = "";
    public string TranslationPrompt { get; set; } = "";
    public double Temperature { get; set; } = 0.2;
    public int RequestTimeoutSeconds { get; set; } = 120;

    public void Normalize()
    {
        Id = string.IsNullOrWhiteSpace(Id) ? Guid.NewGuid().ToString("N") : Id;
        Name = string.IsNullOrWhiteSpace(Name) ? "wlast" : Name.Trim();
        BaseUrl = string.IsNullOrWhiteSpace(BaseUrl) ? "http://127.0.0.1:9573" : BaseUrl.Trim();
        Temperature = Math.Clamp(Math.Round(Temperature, 1), 0, 1);
        RequestTimeoutSeconds = Math.Clamp(RequestTimeoutSeconds, 5, 600);
    }
}
