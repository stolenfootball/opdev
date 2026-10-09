@{
    # Avoid cosmetic churn from every optional rule; retain high-signal diagnostics.
    IncludeRules = @(
        'PSAvoidUsingEmptyCatchBlock', 'PSAvoidUsingInvokeExpression',
        'PSAvoidUsingPlainTextForPassword', 'PSAvoidUsingConvertToSecureStringWithPlainText',
        'PSAvoidUsingUsernameAndPasswordParams', 'PSUseDeclaredVarsMoreThanAssignments',
        'PSUseConsistentIndentation', 'PSUseConsistentWhitespace', 'PSPlaceOpenBrace',
        'PSPlaceCloseBrace'
    )
    Rules = @{
        PSUseConsistentIndentation = @{ Enable = $true; Kind = 'space'; IndentationSize = 4 }
        PSUseConsistentWhitespace = @{ Enable = $true; CheckOpenBrace = $true; CheckOpenParen = $true; CheckOperator = $true; CheckSeparator = $true }
        PSPlaceOpenBrace = @{ Enable = $true; OnSameLine = $true; NewLineAfter = $true }
        PSPlaceCloseBrace = @{ Enable = $true; NewLineAfter = $false }
    }
}
