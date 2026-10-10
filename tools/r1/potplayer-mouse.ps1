# Reads PotPlayer's mouse-action settings from an open Preferences dialog
# through UI Automation, for R1. It only reads: it selects the mouse page in
# the tree, reads every combo box (label, selected entry, all entries) and
# leaves; the caller closes the dialog with Cancel.
#
# usage: powershell -NoProfile -ExecutionPolicy Bypass -File potplayer-mouse.ps1 -Hwnd <dialog> -Out <file.tsv>
param([long]$Hwnd, [string]$Out)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$AE = [System.Windows.Automation.AutomationElement]
$Scope = [System.Windows.Automation.TreeScope]
$Type = [System.Windows.Automation.ControlType]

function Find-All($root, $controlType) {
    $condition = New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, $controlType)
    return $root.FindAll($Scope::Descendants, $condition)
}

$dialog = $AE::FromHandle([IntPtr]$Hwnd)
$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("dialog`t" + $dialog.Current.Name)

# Expand the page tree so that nested pages exist in the UIA tree.
foreach ($pass in 1..3) {
    foreach ($item in (Find-All $dialog $Type::TreeItem)) {
        $pattern = $null
        if ($item.TryGetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern, [ref]$pattern)) {
            if ($pattern.Current.ExpandCollapseState -eq 'Collapsed') { $pattern.Expand() }
        }
    }
}
$pages = Find-All $dialog $Type::TreeItem
$lines.Add("pages`t" + (($pages | ForEach-Object { $_.Current.Name }) -join ' | '))
$mouse = $pages | Where-Object { $_.Current.Name -match 'Mouse' } | Select-Object -First 1
if ($null -eq $mouse) {
    $lines.Add("error`tno page named like Mouse")
    $lines | Set-Content -Encoding UTF8 $Out
    exit 1
}
$select = $mouse.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern)
$select.Select()
Start-Sleep -Milliseconds 1200
$lines.Add("page`t" + $mouse.Current.Name)

$texts = Find-All $dialog $Type::Text | Where-Object { -not $_.Current.IsOffscreen }
$lines.Add("label`tselected`tselected_index`tentries")
foreach ($combo in (Find-All $dialog $Type::ComboBox | Where-Object { -not $_.Current.IsOffscreen })) {
    $box = $combo.Current.BoundingRectangle
    # The label is the nearest text on the same row to the left.
    $label = $texts | Where-Object {
        $r = $_.Current.BoundingRectangle
        ($r.Right -le $box.Left + 2) -and ($r.Top -lt $box.Bottom) -and ($r.Bottom -gt $box.Top)
    } | Sort-Object { $box.Left - $_.Current.BoundingRectangle.Right } | Select-Object -First 1
    $entries = @(Find-All $combo $Type::ListItem | ForEach-Object { $_.Current.Name })
    $selected = ''
    $pattern = $null
    if ($combo.TryGetCurrentPattern([System.Windows.Automation.SelectionPattern]::Pattern, [ref]$pattern)) {
        $chosen = $pattern.Current.GetSelection()
        if ($chosen.Length -gt 0) { $selected = $chosen[0].Current.Name }
    }
    if ($selected -eq '' -and $combo.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$pattern)) {
        $selected = $pattern.Current.Value
    }
    $index = [array]::IndexOf($entries, $selected)
    $name = if ($label) { $label.Current.Name } else { $combo.Current.Name }
    $lines.Add("$name`t$selected`t$index`t" + ($entries -join ' | '))
}
$lines | Set-Content -Encoding UTF8 $Out
