<#
.SYNOPSIS
Imports the Windows Calculator's translations of the strings CalcCraft shows.

.DESCRIPTION
Reads Resources.resw and CEngineStrings.resw of every language in a clone of
microsoft/calculator (MIT licensed, like CalcCraft) and writes locales/<language>.txt:
one "key=value" line per string, in the order of the key list below. Values are trimmed,
and backslashes and line breaks in them are written as \\ and \n.

Keys keep their resw names, except that the XAML attached-property suffixes are
shortened (".[using:Windows.UI.Xaml.Automation]AutomationProperties.Name" becomes
".AutomationName") and the engine's numbered error strings get a "CEngine." prefix.
The unit names and abbreviations are all the UnitName_* and UnitAbbreviation_* keys
that UnitConverterDataLoader.cs uses.

Run it again after adding a key to the list: the source code looks strings up by
these keys (see src/app/strings.rs), and a test checks that every key it uses exists.

.EXAMPLE
pwsh tools/import-strings.ps1 -Source ../win-calculator
#>
param(
    [string]$Source = (Join-Path $PSScriptRoot '..\..\win-calculator')
)

$ErrorActionPreference = 'Stop'

# The strings of Resources.resw that CalcCraft uses.
$keys = @(
    # Modes and the navigation menu
    'StandardModeText'
    'DateCalculationModeText'
    'CategoryName_VolumeText'
    'CategoryName_LengthText'
    'CategoryName_WeightText'
    'CategoryName_TemperatureText'
    'CategoryName_EnergyText'
    'CategoryName_AreaText'
    'CategoryName_SpeedText'
    'CategoryName_TimeText'
    'CategoryName_PowerText'
    'CategoryName_DataText'
    'CategoryName_PressureText'
    'CategoryName_AngleText'
    'CalculatorModeTextCaps'
    'ConverterModeTextCaps'
    'CalculatorModePluralText'
    'ConverterModePluralText'
    'NavCategoryItem_AutomationNameFormat'
    'NavCategoryHeader_AutomationNameFormat'
    'HeaderAutomationName_Calculator'
    'HeaderAutomationName_Converter'
    'HeaderAutomationName_Date'
    # Standard mode
    'num0Button.AutomationName'
    'num1Button.AutomationName'
    'num2Button.AutomationName'
    'num3Button.AutomationName'
    'num4Button.AutomationName'
    'num5Button.AutomationName'
    'num6Button.AutomationName'
    'num7Button.AutomationName'
    'num8Button.AutomationName'
    'num9Button.AutomationName'
    'decimalSeparatorButton.AutomationName'
    'plusButton.AutomationName'
    'minusButton.AutomationName'
    'multiplyButton.AutomationName'
    'divideButton.AutomationName'
    'equalButton.AutomationName'
    'percentButton.AutomationName'
    'clearEntryButton.AutomationName'
    'clearButton.AutomationName'
    'backSpaceButton.AutomationName'
    'invertButton.AutomationName'
    'xpower2Button.AutomationName'
    'squareRootButton.AutomationName'
    'negateButton.AutomationName'
    'converterNegateButton.AutomationName'
    'ClearMemoryButton.AutomationName'
    'MemRecall.AutomationName'
    'MemPlus.AutomationName'
    'MemMinus.AutomationName'
    'memButton.AutomationName'
    'MemoryButton_Open'
    'MemoryButton_Close'
    'HistoryButton_Open'
    'HistoryButton_Close'
    'EnterAlwaysOnTopButton.AutomationName'
    'ExitAlwaysOnTopButton.AutomationName'
    'Format_CalculatorResults'
    'Format_CalculatorResults_Decimal'
    'Format_CalculatorExpression'
    'MemoryPanel.AutomationName'
    'StandardFunctions.AutomationName'
    'DisplayControls.AutomationName'
    'StandardOperators.AutomationName'
    'NumberPad.AutomationName'
    # History and memory
    'HistoryPane'
    'MemoryPane'
    'HistoryLabel.Text'
    'MemoryLabel.Text'
    'DockPanel_HistoryMemoryLists'
    'HistoryEmpty.Text'
    'MemoryPaneEmpty.Text'
    'ClearHistory.AutomationName'
    'ClearMemory.AutomationName'
    'CopyHistoryMenuItem.Text'
    'DeleteHistoryMenuItem.Text'
    'DeleteHistoryMenuItem.AutomationName'
    'ClearMemoryMenuItem.Text'
    'MemPlusMenuItem.Text'
    'MemMinusMenuItem.Text'
    'ClearMemoryItemButton.AutomationName'
    'MemPlusItem.AutomationName'
    'MemMinusItem.AutomationName'
    # Programmer mode
    'ProgrammerModeText'
    'aButton.AutomationName'
    'bButton.AutomationName'
    'cButton.AutomationName'
    'dButton.AutomationName'
    'eButton.AutomationName'
    'fButton.AutomationName'
    'andButton.AutomationName'
    'orButton.AutomationName'
    'notButton.AutomationName'
    'nandButton.AutomationName'
    'norButton.AutomationName'
    'xorButton.AutomationName'
    'lshButton.AutomationName'
    'rshButton.AutomationName'
    'rshLogicalButton.AutomationName'
    'rolButton.AutomationName'
    'rorButton.AutomationName'
    'rolCarryButton.AutomationName'
    'rorCarryButton.AutomationName'
    'modButton.AutomationName'
    'openParenthesisButton.AutomationName'
    'closeParenthesisButton.AutomationName'
    'Format_OpenParenthesisAutomationNamePrefix'
    'bitwiseButton.Text'
    'bitShiftButton.Text'
    'bitwiseButton.AutomationName'
    'bitShiftButton.AutomationName'
    'arithmeticShiftButton.Content'
    'logicalShiftButton.Content'
    'rotateCircularButton.Content'
    'rotateCarryShiftButton.Content'
    'qwordButton.AutomationName'
    'dwordButton.AutomationName'
    'wordButton.AutomationName'
    'byteButton.AutomationName'
    'fullKeypad.AutomationName'
    'bitFlip.AutomationName'
    'Format_HexButtonValue'
    'Format_DecButtonValue'
    'Format_OctButtonValue'
    'Format_BinButtonValue'
    'copyMenuItem'
    'DockPanel_MemoryList'
    'BitFlipItemAutomationName'
    'BitPosition'
    'LeastSignificantBit'
    'MostSignificantBit'
    # Scientific mode
    'ScientificModeText'
    'ScientificAngleOperators.AutomationName'
    'ScientificFunctions.AutomationName'
    'ScientificOperatorPanel.AutomationName'
    'degButton.AutomationName'
    'radButton.AutomationName'
    'gradButton.AutomationName'
    'ftoeButton.AutomationName'
    'shiftButton.AutomationName'
    'piButton.AutomationName'
    'eulerButton.AutomationName'
    'xpower3Button.AutomationName'
    'cubeRootButton.AutomationName'
    'absButton.AutomationName'
    'expButton.AutomationName'
    'factorialButton.AutomationName'
    'powerButton.AutomationName'
    'ySquareRootButton.AutomationName'
    'powerOf10Button.AutomationName'
    'twoPowerXButton.AutomationName'
    'logBase10Button.AutomationName'
    'logBaseY.AutomationName'
    'logBaseEButton.AutomationName'
    'powerOfEButton.AutomationName'
    'trigButton.Text'
    'trigButton.AutomationName'
    'funcButton.Text'
    'funcButton.AutomationName'
    'trigShiftButton.AutomationName'
    'hypButton.AutomationName'
    'sinButton.AutomationName'
    'cosButton.AutomationName'
    'tanButton.AutomationName'
    'secButton.AutomationName'
    'cscButton.AutomationName'
    'cotButton.AutomationName'
    'invsinButton.AutomationName'
    'invcosButton.AutomationName'
    'invtanButton.AutomationName'
    'invsecButton.AutomationName'
    'invcscButton.AutomationName'
    'invcotButton.AutomationName'
    'sinhButton.AutomationName'
    'coshButton.AutomationName'
    'tanhButton.AutomationName'
    'sechButton.AutomationName'
    'cschButton.AutomationName'
    'cothButton.AutomationName'
    'invsinhButton.AutomationName'
    'invcoshButton.AutomationName'
    'invtanhButton.AutomationName'
    'invsechButton.AutomationName'
    'invcschButton.AutomationName'
    'invcothButton.AutomationName'
    'floorButton.AutomationName'
    'ceilButton.AutomationName'
    'randButton.AutomationName'
    'dmsButton.AutomationName'
    'degreesButton.AutomationName'
    # Graphing mode
    'GraphingCalculatorModeText'
    'mathRichEditBox.PlaceholderText'
    'showEquationButtonAutomationName'
    'hideEquationButtonAutomationName'
    'showEquationButtonToolTip'
    'hideEquationButtonToolTip'
    'functionAnalysisButton.AutomationName'
    'functionAnalysisButton.ToolTip'
    'colorChooserButton.AutomationName'
    'colorChooserButton.ToolTip'
    'removeButton.AutomationName'
    'removeButton.ToolTip'
    'cutEquationMenuItem.Text'
    'copyEquationMenuItem.Text'
    'pasteEquationMenuItem.Text'
    'undoEquationMenuItem.Text'
    'selectAllEquationMenuItem.Text'
    'functionAnalysisMenuItem'
    'colorChooserMenuItem'
    'removeMenuItem'
    'LineOptionsHeading.Text'
    'LineColorText.Text'
    'StyleChooserBoxHeading.Text'
    'solidLineStyleAutomationName'
    'dashLineStyleAutomationName'
    'dotLineStyleAutomationName'
    'equationColor1AutomationName'
    'equationColor2AutomationName'
    'equationColor3AutomationName'
    'equationColor4AutomationName'
    'equationColor5AutomationName'
    'equationColor6AutomationName'
    'equationColor7AutomationName'
    'equationColor8AutomationName'
    'equationColor9AutomationName'
    'equationColor10AutomationName'
    'equationColor11AutomationName'
    'equationColor12AutomationName'
    'equationColor13AutomationName'
    'equationColor14BlackAutomationName'
    'equationColor14WhiteAutomationName'
    'GraphSwitchToEquationMode'
    'GraphSwitchToGraphMode'
    'graphAutomationName'
    'enableTracingButtonToolTip'
    'disableTracingButtonToolTip'
    'shareButton.AutomationName'
    'shareButton.ToolTip'
    'GraphImageAltText'
    'EquationsShareHeader'
    'VariablesShareHeader'
    'ShareActionErrorMessage'
    'ShareActionErrorOk'
    'graphSettingsButton.AutomationName'
    'graphSettingsButton.ToolTip'
    'zoomInButton.AutomationName'
    'zoomInButton.ToolTip'
    'zoomOutButton.AutomationName'
    'zoomOutButton.ToolTip'
    'graphViewButton.AutomationName'
    'graphViewButton.ToolTip'
    'GraphCopyMenuItem.Text'
    'GraphOptionsHeading.Text'
    'GridHeading.Text'
    'ResetViewButton.Content'
    'GraphSettingsXMin.Header'
    'GraphSettingsXMax.Header'
    'GraphSettingsYMin.Header'
    'GraphSettingsYMax.Header'
    'UnitsHeading.Text'
    'TrigModeRadians.Content'
    'TrigModeDegrees.Content'
    'TrigModeGradians.Content'
    'LineThicknessBoxHeading.Text'
    'SmallLineWidthAutomationName'
    'MediumLineWidthAutomationName'
    'LargeLineWidthAutomationName'
    'ExtraLargeLineWidthAutomationName'
    'GraphThemeHeading.Text'
    'AlwaysLightTheme.Content'
    'MatchAppTheme.Content'
    'VariableValueTextBox.AutomationName'
    'VariableValueSlider.AutomationName'
    'VariableMinTextBox.AutomationName'
    'VariableStepTextBox.AutomationName'
    'VariableMaxTextBox.AutomationName'
    'MinTextBlock.Text'
    'StepTextBlock.Text'
    'MaxTextBlock.Text'
    'VariableAreaSettings.AutomationName'
    'VariableAreaSettings.ToolTip'
    'inequalityButton.Text'
    'inequalityButton.AutomationName'
    'lessThanFlyoutButton.AutomationName'
    'lessThanOrEqualFlyoutButton.AutomationName'
    'equalsFlyoutButton.AutomationName'
    'greaterThanOrEqualFlyoutButton.AutomationName'
    'greaterThanFlyoutButton.AutomationName'
    'xButton.AutomationName'
    'yButton.AutomationName'
    'graphingEqualButton.AutomationName'
    'submitButton.AutomationName'
    # Graphing mode: function analysis
    'KeyGraphFeaturesLabel.Text'
    'equationAnalysisBack.AutomationName'
    'equationAnalysisBack.ToolTip'
    'Domain'
    'Range'
    'XIntercept'
    'YIntercept'
    'Minima'
    'Maxima'
    'InflectionPoints'
    'VerticalAsymptotes'
    'HorizontalAsymptotes'
    'ObliqueAsymptotes'
    'Parity'
    'Periodicity'
    'Monotonicity'
    'KGFDomainNone'
    'KGFRangeNone'
    'KGFXInterceptNone'
    'KGFYInterceptNone'
    'KGFMinimaNone'
    'KGFMaximaNone'
    'KGFInflectionPointsNone'
    'KGFVerticalAsymptotesNone'
    'KGFHorizontalAsymptotesNone'
    'KGFObliqueAsymptotesNone'
    'KGFParityOdd'
    'KGFParityEven'
    'KGFParityNeither'
    'KGFParityUnknown'
    'KGFMonotonicityIncreasing'
    'KGFMonotonicityDecreasing'
    'KGFMonotonicityConstant'
    'KGFMonotonicityError'
    'KGFTooComplexFeaturesError'
    'KGFAnalysisNotSupported'
    'KGFVariableIsNotX'
    # Graphing mode: equation errors
    'ParenthesisMismatch'
    'UnmatchedParenthesis'
    'TooManyDecimalPoints'
    'DecimalPointWithoutDigits'
    'UnexpectedEndOfExpression'
    'UnexpectedToken'
    'InvalidToken'
    'TooManyEquals'
    'EqualWithoutGraphVariable'
    'InvalidEquationSyntax'
    'EmptyExpression'
    'IncorrectNumParameter'
    'CannotUseIInReal'
    # Date calculation
    'DateCalculationOption.AutomationName'
    'Date_DifferenceOption.Content'
    'Date_AddSubtractOption.Content'
    'DateDiff_FromHeader.Header'
    'DateDiff_ToHeader.Header'
    'AddSubtract_Date_FromHeader.Header'
    'Date_DifferenceLabel.Text'
    'DateLabel.Text'
    'AddOption.Content'
    'SubtractOption.Content'
    'YearsLabel.Text'
    'MonthsLabel.Text'
    'DaysLabel.Text'
    'Date_SameDates'
    'Date_OutOfBoundMessage'
    'Date_Year'
    'Date_Years'
    'Date_Month'
    'Date_Months'
    'Date_Week'
    'Date_Weeks'
    'Date_Day'
    'Date_Days'
    'Date_DifferenceResultAutomationName'
    'Date_ResultingDateAutomationName'
    # Converters
    'SupplementaryResultsHeader.Text'
    'SupplementaryUnit_AutomationName'
    'Format_ValueFrom'
    'Format_ValueTo'
    'InputUnit_Name'
    'OutputUnit_Name'
    # Settings
    'SettingsHeader.Text'
    'TitleBarBackButton.AutomationName'
    'SettingsAppearance.Text'
    'AppThemeExpander.Header'
    'AppThemeExpander.Description'
    'LightThemeRadioButton.Content'
    'DarkThemeRadioButton.Content'
    'SystemThemeRadioButton.Content'
    'AboutGroupTitle.Text'
    'FeedbackButton.Content'
)

# The ordinal numbers the bit toggling keypad names its bits by ("2nd bit"), keys 1 to 63.
$keys += 1..63 | ForEach-Object { "$_" }

# The engine's error messages in CEngineStrings.resw: divide by zero, invalid input,
# undefined result, overflow, no result (a shift by the word size or more).
$engineKeys = @('99', '100', '101', '107', '108')

$suffixes = @{
    '.[using:Windows.UI.Xaml.Automation]AutomationProperties.Name' = '.AutomationName'
    '.[using:Windows.UI.Xaml.Controls]ToolTipService.ToolTip' = '.ToolTip'
}

function Read-Resw([string]$path) {
    $table = @{}
    # [xml] skips the example <data> elements of the header comment.
    $xml = [xml](Get-Content -LiteralPath $path -Raw -Encoding utf8)
    foreach ($data in $xml.root.data) {
        $name = $data.name
        foreach ($long in $suffixes.Keys) {
            if ($name.EndsWith($long)) {
                $name = $name.Substring(0, $name.Length - $long.Length) + $suffixes[$long]
            }
        }
        $table[$name] = [string]$data.value
    }
    return $table
}

# Values are trimmed: a few have a stray space at the end ("Millimeters of mercury ").
function Escape([string]$value) {
    return $value.Trim().Replace('\', '\\').Replace("`r", '').Replace("`n", '\n')
}

$resources = Join-Path $Source 'src\Calculator\Resources'
$loader = Join-Path $Source 'src\Calculator.ViewModels\DataLoaders\UnitConverterDataLoader.cs'
if (-not (Test-Path $resources) -or -not (Test-Path $loader)) {
    throw "No Windows Calculator clone at $Source (pass -Source)."
}

# Some keys have hyphens, and the loader writes MillimeterOfMercury's with a trailing space.
$unitKeys = [regex]::Matches((Get-Content -LiteralPath $loader -Raw), '"(Unit(?:Name|Abbreviation)_[^"]+)"') |
    ForEach-Object { $_.Groups[1].Value.Trim() } |
    Sort-Object -Unique

$out = Join-Path $PSScriptRoot '..\locales'
New-Item -ItemType Directory -Force $out | Out-Null
$utf8 = New-Object System.Text.UTF8Encoding $false

$languages = Get-ChildItem -LiteralPath $resources -Directory | Sort-Object Name
foreach ($language in $languages) {
    $strings = Read-Resw (Join-Path $language.FullName 'Resources.resw')
    $engine = Read-Resw (Join-Path $language.FullName 'CEngineStrings.resw')
    $lines = [System.Collections.Generic.List[string]]::new()
    $lines.Add("# $($language.Name): generated by tools/import-strings.ps1 from the Windows Calculator")
    $lines.Add('# (microsoft/calculator, MIT license, Copyright (c) Microsoft Corporation). Do not edit.')
    foreach ($key in $keys + $unitKeys) {
        if (-not $strings.ContainsKey($key)) {
            throw "$($language.Name): no $key in Resources.resw"
        }
        $lines.Add("$key=$(Escape $strings[$key])")
    }
    foreach ($key in $engineKeys) {
        if (-not $engine.ContainsKey($key)) {
            throw "$($language.Name): no $key in CEngineStrings.resw"
        }
        $lines.Add("CEngine.$key=$(Escape $engine[$key])")
    }
    $file = Join-Path $out "$($language.Name).txt"
    [System.IO.File]::WriteAllText($file, ($lines -join "`n") + "`n", $utf8)
}
Write-Host "Wrote $($languages.Count) languages with $($keys.Count + $unitKeys.Count + $engineKeys.Count) strings each to $out"
