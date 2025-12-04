using Avalonia.Controls;

namespace MyAvaloniaApp
{
    public partial class MainWindow : Window
    {
        public MainWindow()
        {
            InitializeComponent();
        }

        public void SetLabel1Text(string text)
        {
            Label1.Text = text;
            Label1.Background = Avalonia.Media.Brushes.DarkRed;
            Label1.Foreground = Avalonia.Media.Brushes.White;
        }

        public void SetLabel2Text(string text)
        {
            Label2.Text = text;
        }
    }
}