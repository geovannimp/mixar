// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Portuguese (`pt`).
class AppLocalizationsPt extends AppLocalizations {
  AppLocalizationsPt([String locale = 'pt']) : super(locale);

  @override
  String get settingsTitle => 'CONFIGURAÇÕES';

  @override
  String get settingsSubtitle =>
      'Salvar reinicia a engine automaticamente se ela estiver em execução.';

  @override
  String get settingsLoading => 'Carregando configurações…';

  @override
  String get settingsCloseSemantics => 'Fechar';

  @override
  String get settingsSave => 'Salvar';

  @override
  String get settingsSaving => 'Salvando…';

  @override
  String get settingsSavedToast => 'Configurações salvas';

  @override
  String get settingsSavedNotAppliedToast =>
      'Configurações salvas, mas não aplicadas';

  @override
  String get settingsSaveFailedToast => 'Falha ao salvar';

  @override
  String get settingsUnsavedTitle => 'Configurações não salvas';

  @override
  String get settingsUnsavedBody => 'Salvar alterações antes de fechar?';

  @override
  String get settingsCancel => 'Cancelar';

  @override
  String get settingsDiscard => 'Descartar';

  @override
  String get commonLoading => 'Carregando…';

  @override
  String get commonSync => 'Sincronizar';

  @override
  String get commonClear => 'Limpar';

  @override
  String get commonUpdate => 'Atualizar';

  @override
  String get commonImport => 'Importar';

  @override
  String get commonNone => 'Nenhum';

  @override
  String get settingsSectionAudio => 'Áudio';

  @override
  String get settingsSectionMixer => 'Mixer';

  @override
  String get settingsSectionWaveform => 'Forma de onda';

  @override
  String get settingsSectionDeck => 'Deck';

  @override
  String get settingsSectionUi => 'Interface';

  @override
  String get settingsSectionLayout => 'Layout';

  @override
  String get settingsSectionLibrary => 'Biblioteca';

  @override
  String get settingsSectionIntegrations => 'Integrações';

  @override
  String get settingsSectionStorage => 'Armazenamento';

  @override
  String get settingsSectionSession => 'Sessão';

  @override
  String get settingsSectionControllers => 'Controladores';

  @override
  String get settingsSectionPrivacy => 'Privacidade';

  @override
  String get settingsPrivacyDescription =>
      'Dados anónimos de utilização da aplicação.';

  @override
  String get settingsLayoutDescription =>
      'Quais painéis o mixer mostra, onde fica a forma de onda e como a biblioteca abre.';

  @override
  String get settingsLayoutPresetTitle => 'Predefinições';

  @override
  String get settingsLayoutPresetDescription =>
      'Comece por um dispositivo e depois mostre ou oculte cada painel.';

  @override
  String get settingsLayoutPreset => 'Predefinição';

  @override
  String get settingsLayoutPresetDesktop => 'Desktop';

  @override
  String get settingsLayoutPresetDesktopSubtitle =>
      'Forma de onda, faixa do mixer, barra de FX e biblioteca acoplada.';

  @override
  String get settingsLayoutPresetTablet => 'Tablet';

  @override
  String get settingsLayoutPresetTabletSubtitle =>
      'Forma de onda, faixa do mixer, barra de FX e biblioteca numa gaveta.';

  @override
  String get settingsLayoutPresetMobile => 'Telemóvel';

  @override
  String get settingsLayoutPresetMobileSubtitle =>
      'Decks com a forma de onda entre eles e a biblioteca numa gaveta. Sem faixa do mixer.';

  @override
  String get settingsLayoutPresetCustom => 'Personalizado';

  @override
  String get settingsLayoutPresetCustomSubtitle =>
      'Nenhuma predefinição corresponde aos painéis visíveis abaixo.';

  @override
  String get settingsLayoutMixerTitle => 'Faixa do mixer';

  @override
  String get settingsLayoutMixerDescription =>
      'Canais, faders, EQ, crossfader e medidores entre os decks. Desligar alarga os decks para uma fila única ou entrega a coluna à forma de onda centrada.';

  @override
  String get settingsLayoutMixerEnable => 'Mostrar faixa do mixer';

  @override
  String get settingsLayoutShowCrossfader => 'Crossfader';

  @override
  String get settingsLayoutMixerMeters => 'Medidores do mixer';

  @override
  String get settingsLayoutMixerMetersNone => 'Nenhum';

  @override
  String get settingsLayoutMixerMetersNoneSubtitle =>
      'Sem medidores nos decks; as colunas de canal dispensam a faixa do medidor.';

  @override
  String get settingsLayoutMixerMetersMono => 'Mono';

  @override
  String get settingsLayoutMixerMetersStereo => 'Estéreo';

  @override
  String get settingsLayoutMixerMetersMonoSubtitle =>
      'Uma escada por deck (máximo de esquerda/direita).';

  @override
  String get settingsLayoutMixerMetersStereoSubtitle =>
      'Escadas esquerda e direita separadas por deck.';

  @override
  String get settingsLayoutFxBarTitle => 'Barra de FX';

  @override
  String get settingsLayoutFxBarDescription =>
      'A unidade de efeitos partilhada e o medidor master no fundo do painel do mixer. Os níveis master e de auscultadores abrem a partir do medidor.';

  @override
  String get settingsLayoutFxBarEnable => 'Mostrar barra de FX';

  @override
  String get settingsLayoutDeckCount => 'Layout dos decks';

  @override
  String get settingsLayoutDeckCountDescription =>
      'Quantos decks o mixer mostra. Alterar isso reinicia o mecanismo de áudio e recarrega as faixas.';

  @override
  String get settingsLayoutDeckCountDecks => 'Decks';

  @override
  String get settingsLayoutDeckCountTwo => '2 decks';

  @override
  String get settingsLayoutDeckCountFour => '4 decks';

  @override
  String get settingsLayoutWaveformTitle => 'Posição da forma de onda';

  @override
  String get settingsLayoutWaveformDescription =>
      'Em cima, a forma de onda fica acima dos decks; em baixo, passa entre os decks e a biblioteca. No meio, substitui o mixer central pelas pistas. Desliga o interruptor para esconder a forma de onda.';

  @override
  String get settingsLayoutWaveformPosition => 'Posição';

  @override
  String get settingsLayoutShowWaveform => 'Mostrar forma de onda';

  @override
  String get settingsLayoutWaveformTop => 'Em cima';

  @override
  String get settingsLayoutWaveformMiddleHorizontal => 'Meio horizontal';

  @override
  String get settingsLayoutWaveformMiddleVertical => 'Meio vertical';

  @override
  String get settingsLayoutWaveformBottom => 'Em baixo';

  @override
  String get settingsLayoutLibraryDrawerTitle => 'Gaveta da biblioteca';

  @override
  String get settingsLayoutLibraryDrawerDescription =>
      'Onde a biblioteca abre. O modo gaveta oculta a biblioteca fixa e a abre sobre o mixer.';

  @override
  String get settingsLayoutLibraryDrawerEnable => 'Biblioteca como gaveta';

  @override
  String get settingsLayoutLibraryDrawerPosition => 'Posição';

  @override
  String get settingsLayoutLibraryDrawerLeft => 'Esquerda';

  @override
  String get settingsLayoutLibraryDrawerRight => 'Direita';

  @override
  String get settingsLayoutLibraryDrawerTop => 'Topo';

  @override
  String get settingsLayoutLibraryDrawerBottom => 'Inferior';

  @override
  String get settingsUiDescription =>
      'Tema, idioma, dicas ao passar o mouse e apresentação dos controles.';

  @override
  String get settingsUiChromeTitle => 'Chrome e controles';

  @override
  String get settingsUiChromeDescription =>
      'Idioma, dicas ao passar o mouse e como os seletores abrem.';

  @override
  String get settingsUiThemeTitle => 'Tema';

  @override
  String get settingsUiThemeDescription =>
      'Brilho e a cor de destaque usada pela interface da marca.';

  @override
  String get settingsUiTheme => 'Tema';

  @override
  String get settingsUiThemeLight => 'Claro';

  @override
  String get settingsUiThemeDark => 'Escuro';

  @override
  String get settingsUiAccentColor => 'Cor de destaque';

  @override
  String get settingsUiAccentTeal => 'Turquesa';

  @override
  String get settingsUiAccentBlue => 'Azul';

  @override
  String get settingsUiAccentViolet => 'Violeta';

  @override
  String get settingsUiAccentOrange => 'Laranja';

  @override
  String get settingsUiAccentPink => 'Rosa';

  @override
  String get settingsUiLanguageLabel => 'Idioma';

  @override
  String get settingsSystemDefault => 'Padrão do sistema';

  @override
  String get settingsLanguageEn => 'English';

  @override
  String get settingsLanguagePtBr => 'Português (Brasil)';

  @override
  String get settingsShowTooltips => 'Mostrar dicas';

  @override
  String get settingsUiOnboardingTitle => 'Configuração inicial';

  @override
  String get settingsUiOnboardingDescription =>
      'Mostrar outra vez os passos de boas-vindas em vez do mixer. Salvo com o resto das configurações.';

  @override
  String get settingsUiOnboardingReplay => 'Mostrar configuração outra vez';

  @override
  String onboardingStepProgress(int index, int total) {
    return 'Passo $index de $total';
  }

  @override
  String get onboardingSkip => 'Pular configuração';

  @override
  String get onboardingBack => 'Voltar';

  @override
  String get onboardingNext => 'Continuar';

  @override
  String get onboardingSaveFailedToast =>
      'Não foi possível salvar a configuração';

  @override
  String get onboardingWelcomeTitle => 'Bem-vindo ao Mixar';

  @override
  String get onboardingWelcomeBody =>
      'Só alguns passos para deixar o Mixar com o seu estilo.';

  @override
  String get onboardingAppearanceTitle => 'Deixe o app com a sua cara';

  @override
  String get onboardingAppearanceBody =>
      'Escolha um tema e uma cor de destaque. Os dois repintam o app conforme você escolhe.';

  @override
  String get onboardingLibraryTitle => 'Traga a sua música';

  @override
  String get onboardingLibraryBody =>
      'Aponte o Mixar para uma pasta de faixas ou importe a biblioteca de outro app de DJ. Os arquivos de áudio nunca são movidos.';

  @override
  String get onboardingLibraryChooseFolder => 'Escolher pasta de música';

  @override
  String get onboardingLibraryFolderAdded => 'Pasta adicionada';

  @override
  String get onboardingLibraryFolderFailed =>
      'Não foi possível adicionar essa pasta';

  @override
  String get onboardingPrivacyTitle => 'Os seus dados ficam aqui';

  @override
  String get onboardingPrivacyBody =>
      'Desativado por padrão. Envia eventos anônimos e relatórios de erro; os caminhos de arquivos são removidos e a sua música nunca é enviada.';

  @override
  String get onboardingControllersTitle => 'Configure o seu controlador';

  @override
  String get onboardingControllersTrusted =>
      'Dispositivos confiáveis conectam automaticamente na próxima sessão.';

  @override
  String get onboardingControllersMappings =>
      'Os mapeamentos ficam em Configurações → Controladores.';

  @override
  String get onboardingKickerSetup => 'Configuração';

  @override
  String get onboardingKickerAppearance => 'Aparência';

  @override
  String get onboardingKickerLibrary => 'Biblioteca';

  @override
  String get onboardingKickerPrivacy => 'Privacidade';

  @override
  String get onboardingKickerControllers => 'Controladores';

  @override
  String get onboardingLibraryImportLabel =>
      'Ou importe uma biblioteca existente';

  @override
  String get onboardingPrivacyOffTitle => 'Manter privado';

  @override
  String get onboardingPrivacyOffBody =>
      'Sem análises. Nada sai desta máquina.';

  @override
  String get onboardingPrivacyOnTitle => 'Compartilhar uso anônimo';

  @override
  String get onboardingPrivacyOnBody =>
      'Eventos anônimos e relatórios de erro. Sem música, sem nomes de arquivos.';

  @override
  String get onboardingLanguageFollowsOs => 'Segue o idioma do sistema';

  @override
  String get onboardingLibraryFolderHint =>
      'O Mixar lê os arquivos onde eles estão. Nada é movido ou copiado.';

  @override
  String onboardingLibraryFolderReady(String name) {
    return '“$name” adicionada. Escolha outra pasta se quiser mais.';
  }

  @override
  String get onboardingLibraryBrowse => 'Escolher';

  @override
  String get onboardingReadyTitle => 'Está tudo pronto';

  @override
  String get onboardingReadyBody =>
      'Salve estas escolhas e o mixer abre pronto para tocar.';

  @override
  String get onboardingReadyFootnote =>
      'Nada aqui é definitivo — todas as escolhas ficam nas Configurações.';

  @override
  String get onboardingStart => 'Começar a mixar';

  @override
  String get onboardingTipLoadTitle => 'Carregue a primeira faixa';

  @override
  String get onboardingTipLoadBody =>
      'Arraste um arquivo para um deck ou dê um duplo clique numa linha da biblioteca.';

  @override
  String get onboardingTipAnalyzeTitle => 'A análise corre sozinha';

  @override
  String get onboardingTipAnalyzeBody =>
      'Grade de batida, forma de onda, tonalidade e BPM são escritos assim que a faixa entra na biblioteca.';

  @override
  String get onboardingTipSettingsTitle => 'Ajuste depois';

  @override
  String get onboardingTipSettingsBody =>
      'Áudio, controladores, layout e forma de onda estão todos nas Configurações.';

  @override
  String get onboardingKickerReady => 'Pronto';

  @override
  String onboardingControllersDetected(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '$count controladores encontrados. Confie nos que você quer conectar automaticamente.',
      one: 'Um controlador encontrado. Confie nele para conectar automaticamente.',
    );
    return '$_temp0';
  }

  @override
  String onboardingControllersMapping(String name) {
    return 'Mapeamento: $name';
  }

  @override
  String get onboardingControllersTrust => 'Confiar';

  @override
  String get onboardingControllersTrustedAction => 'De confiança';

  @override
  String get onboardingFloatDecks => 'Dois decks';

  @override
  String get onboardingFloatWaveform => 'Forma de onda';

  @override
  String get onboardingFloatStems => 'Stems';

  @override
  String get onboardingFloatMixer => 'Mixer';

  @override
  String get onboardingFloatJog => 'Jog';

  @override
  String get onboardingFloatSync => 'Sync';

  @override
  String get onboardingFloatCue => 'Cue';

  @override
  String get onboardingFloatLoop => 'Loop';

  @override
  String get onboardingFloatFx => 'FX';

  @override
  String get onboardingFloatLibrary => 'Biblioteca';

  @override
  String get onboardingKickerLanguage => 'Idioma';

  @override
  String get onboardingLanguageTitle => 'Escolha o seu idioma';

  @override
  String get onboardingLanguageBody =>
      'O Mixar segue o idioma do sistema, a menos que você escolha um. O app inteiro muda assim que você escolhe.';

  @override
  String get onboardingFloatBpm => 'BPM';

  @override
  String get onboardingFloatKeys => 'Tonalidades';

  @override
  String get onboardingFloatCues => 'Hot cues';

  @override
  String get onboardingFloatAnalyze => 'Analisar';

  @override
  String get settingsUiSelectStyle => 'Estilo de seleção';

  @override
  String get settingsUiSelectStyleAuto => 'Automático';

  @override
  String get settingsUiSelectStyleDesktop => 'Desktop';

  @override
  String get settingsUiSelectStyleMobile => 'Mobile';

  @override
  String get settingsUiSelectStyleAutoSubtitle =>
      'Padrão da plataforma — SO desktop usa popovers, celulares usam diálogos.';

  @override
  String get settingsUiSelectStyleDesktopSubtitle =>
      'Popovers ancorados para seletores e menus ⋯.';

  @override
  String get settingsUiSelectStyleMobileSubtitle =>
      'Abre seletores e menus ⋯ em um diálogo.';

  @override
  String get settingsAudioDescription => 'Saída do engine e barramentos.';

  @override
  String get settingsAudioEngineTitle => 'Engine';

  @override
  String get settingsAudioEngineDescription => 'Resampler e tamanho do buffer.';

  @override
  String get settingsAudioLowLatency => 'Baixa latência';

  @override
  String get settingsAudioSampleRate => 'Taxa de amostragem';

  @override
  String get settingsAudioSampleRateHint =>
      'O padrão do dispositivo segue a taxa em que o dispositivo de saída mixa.';

  @override
  String settingsAudioSampleRateHz(int rate) {
    return '$rate Hz';
  }

  @override
  String get settingsAudioSampleRateDeviceDefault => 'Padrão do dispositivo';

  @override
  String settingsAudioSampleRateDeviceDefaultWithRate(int rate) {
    return 'Padrão do dispositivo ($rate Hz)';
  }

  @override
  String get settingsAudioSampleRateLoading =>
      'Carregando taxas deste dispositivo…';

  @override
  String get settingsAudioResamplerQuality => 'Qualidade do resampler';

  @override
  String get settingsAudioResamplerLow => 'Baixa';

  @override
  String get settingsAudioResamplerMedium => 'Média';

  @override
  String get settingsAudioResamplerHigh => 'Alta';

  @override
  String get settingsAudioBufferSize => 'Tamanho do buffer';

  @override
  String get settingsAudioBufferSizeHint =>
      'Deve ser múltiplo de 64 frames (tamanho do chunk do mixer).';

  @override
  String settingsAudioBufferSizeSemantics(int frames) {
    return '$frames frames';
  }

  @override
  String get settingsAudioMasterBus => 'Barramento master';

  @override
  String get settingsAudioMasterBusDescription =>
      'Dispositivo de saída principal e roteamento de canais.';

  @override
  String get settingsAudioPreviewBusTitle => 'Barramento de escuta';

  @override
  String get settingsAudioPreviewBusDescription =>
      'Saída de fones / cue quando a escuta está ativada.';

  @override
  String get settingsAudioPreviewBus => 'Ativar barramento de escuta';

  @override
  String settingsAudioControllerOverride(String controller) {
    return 'Substituído pelo mapeamento $controller';
  }

  @override
  String get settingsAudioDevice => 'Dispositivo';

  @override
  String get settingsAudioChannelMode => 'Modo de canal';

  @override
  String get settingsAudioStereoPair => 'Par stereo';

  @override
  String get settingsAudioMonoFold => 'Mono (somar L+R)';

  @override
  String get settingsAudioLeftChannel => 'Canal esquerdo';

  @override
  String get settingsAudioRightChannel => 'Canal direito';

  @override
  String get settingsAudioLoadingDevices => 'Carregando dispositivos…';

  @override
  String get settingsMixerDescription =>
      'Nivelamento de volume, roteamento do sampler e fallback de stems em tempo real.';

  @override
  String get settingsMixerNormalizerTitle => 'Normalizador de volume';

  @override
  String get settingsMixerNormalizerDescription =>
      'Alinha faixas analisadas a um volume alvo.';

  @override
  String get settingsMixerVolumeNormalizer => 'Normalizador de volume';

  @override
  String get settingsMixerTargetLufs => 'LUFS alvo';

  @override
  String get settingsMixerSamplerRoutingTitle => 'Roteamento do sampler';

  @override
  String get settingsMixerSamplerRoutingDescription =>
      'Onde os hits do sampler entram no caminho de sinal, em relação ao channel strip.';

  @override
  String get settingsMixerSamplerStripRoute => 'Rota do sampler no strip';

  @override
  String get settingsMixerSamplerStripBefore => 'Antes do channel strip';

  @override
  String get settingsMixerSamplerStripAfter => 'Depois do channel strip';

  @override
  String get settingsMixerRealtimeStemsTitle => 'Stems em tempo real';

  @override
  String get settingsMixerRealtimeStemsDescription =>
      'Separa faixas sem cache de stems durante a reprodução. Faixas com stems offline têm sempre prioridade.';

  @override
  String get settingsMixerRealtimeStemsNotRecommended => 'Não recomendado.';

  @override
  String get settingsMixerRealtimeStemsWarning =>
      'A qualidade é menor e usa um núcleo de CPU.';

  @override
  String get settingsWaveformDescription =>
      'RGB mistura grave/médio/agudo em uma cor. Filtrado empilha as três bandas.';

  @override
  String get settingsWaveformDisplayTitle => 'Exibição';

  @override
  String get settingsWaveformDisplayDescription =>
      'RGB mistura grave/médio/agudo em uma cor. Filtrado empilha as três bandas.';

  @override
  String get settingsWaveformDisplayMode => 'Modo de exibição';

  @override
  String get settingsWaveformModeRgb => 'RGB';

  @override
  String get settingsWaveformModeFiltered => 'Filtrado';

  @override
  String get settingsDeckDescription =>
      'Comportamento padrão de jog, tempo, travão e sampler para novos decks.';

  @override
  String get settingsDeckJogTitle => 'Jog wheel';

  @override
  String get settingsDeckJogDescription =>
      'Padrões da política do prato superior (toque) e externo (freewheel).';

  @override
  String get settingsDeckTopJogMode => 'Modo do jog superior';

  @override
  String get settingsDeckOuterJogMode => 'Modo do jog externo';

  @override
  String get settingsDeckJogVinyl => 'Vinil (scratch)';

  @override
  String get settingsDeckJogPitchBend => 'Pitch bend';

  @override
  String get settingsDeckJogIgnore => 'Ignorar';

  @override
  String get settingsDeckTempoKeyTitle => 'Tempo e tom';

  @override
  String get settingsDeckTempoKeyDescription =>
      'Faixa padrão do pitch fader e key lock para novos decks.';

  @override
  String get settingsDeckDefaultTempoRange => 'Faixa de tempo padrão';

  @override
  String get settingsDeckDefaultKeyLock => 'Key lock padrão';

  @override
  String get settingsDeckDefaultKeyLockHint =>
      'Com ligado, só altera o tempo (time-stretch). Desligado = tempo de vinil.';

  @override
  String get settingsDeckKeyLock => 'Key lock';

  @override
  String get settingsDeckVinylBrakePause => 'Freio de vinil no pause';

  @override
  String get settingsDeckVinylBrakePauseHint =>
      'Pause desacelera o deck como toca-discos, em vez de cortar na hora.';

  @override
  String get settingsDeckVinylBrakePauseToggle => 'Frear no pause';

  @override
  String get settingsDeckVinylBrakeStopTime => 'Tempo de parada';

  @override
  String get settingsDeckVinylBrakeStopTimeHint =>
      'Quanto tempo o prato leva para parar após o pause.';

  @override
  String settingsStepperDecrease(String field) {
    return 'Diminuir $field';
  }

  @override
  String settingsStepperIncrease(String field) {
    return 'Aumentar $field';
  }

  @override
  String get settingsDeckSamplerTitle => 'Sampler';

  @override
  String get settingsDeckSamplerDescription =>
      'Modo de reprodução padrão para bancos herdados e banco padrão por deck.';

  @override
  String get settingsDeckSamplerPlayMode => 'Modo de reprodução do sampler';

  @override
  String get settingsDeckSamplerPlayOneshot => 'Oneshot';

  @override
  String get settingsDeckSamplerPlayHold => 'Hold';

  @override
  String get settingsDeckSamplerPlayLoop => 'Loop';

  @override
  String get settingsDeckDefaultSamplerBankA =>
      'Banco padrão do sampler no deck A';

  @override
  String get settingsDeckDefaultSamplerBankB =>
      'Banco padrão do sampler no deck B';

  @override
  String get settingsDeckDefaultSamplerBankC =>
      'Banco padrão do sampler no deck C';

  @override
  String get settingsDeckDefaultSamplerBankD =>
      'Banco padrão do sampler no deck D';

  @override
  String get settingsLayoutDeckCountRestartTitle => 'Alterar layout dos decks?';

  @override
  String get settingsLayoutDeckCountRestartBody =>
      'Alterar a quantidade de decks reinicia o mecanismo de áudio e recarrega as faixas. Faixas em decks removidos ao mudar para 2 decks não serão restauradas.';

  @override
  String get settingsLayoutDeckCountRestartConfirm => 'Reiniciar e aplicar';

  @override
  String get settingsLibraryListTitle => 'Lista da biblioteca';

  @override
  String get settingsLibraryListDescription =>
      'Como as faixas aparecem na tabela da biblioteca.';

  @override
  String get settingsLibraryDescription =>
      'Análise offline, metadados e exibição da lista.';

  @override
  String get settingsIntegrationsDescription =>
      'Importe a biblioteca de outro aplicativo de DJ.';

  @override
  String get settingsLibraryAnalysisTitle => 'Análise';

  @override
  String get settingsLibraryAnalysisDescription =>
      'Quão profundamente a análise offline examina cada faixa.';

  @override
  String get settingsLibraryAnalysisQuality => 'Qualidade da análise';

  @override
  String get settingsLibraryAnalysisFast => 'Rápida';

  @override
  String get settingsLibraryAnalysisFastSubtitle =>
      'Analisa um trecho curto para varreduras rápidas da biblioteca.';

  @override
  String get settingsLibraryAnalysisPrecise => 'Precisa';

  @override
  String get settingsLibraryAnalysisPreciseSubtitle =>
      'Análise equilibrada para a maioria das bibliotecas.';

  @override
  String get settingsLibraryAnalysisComplete => 'Completa';

  @override
  String get settingsLibraryAnalysisCompleteSubtitle =>
      'Analisa a faixa inteira (mais lenta e precisa).';

  @override
  String get settingsLibraryTagWriteBackTitle => 'Tags dos arquivos';

  @override
  String get settingsLibraryTagWriteBackDescription =>
      'O Mixar guarda BPM e tom no próprio banco. Quando ativado, também grava esses valores nas tags de cada arquivo, para que outros softwares de DJ vejam o mesmo. Desativado por padrão — o Mixar nunca altera seus arquivos enquanto estiver desligado.';

  @override
  String get settingsLibraryTagWriteBack =>
      'Gravar BPM e tom nas tags dos arquivos';

  @override
  String get settingsLibraryMusicalKeyTitle => 'Tom musical';

  @override
  String get settingsLibraryMusicalKeyDescription =>
      'Como os tons são rotulados e coloridos no deck e na tabela da biblioteca.';

  @override
  String get settingsLibraryKeyDisplayMode => 'Modo de exibição do tom';

  @override
  String get settingsLibraryKeyDisplayMusical => 'Musical';

  @override
  String get settingsLibraryKeyDisplayMusicalSubtitle =>
      'Nomes das notas no chip do deck e na coluna de tom — ex.: C, Am, F#m.';

  @override
  String get settingsLibraryKeyDisplayCamelot => 'Camelot';

  @override
  String get settingsLibraryKeyDisplayCamelotSubtitle =>
      'Códigos Mixed In Key — ex.: 8B (Dó maior), 8A (Lá menor), 11B.';

  @override
  String get settingsLibraryKeyColorMode => 'Modo de cor do tom';

  @override
  String get settingsLibraryKeyColorOff => 'Desligado';

  @override
  String get settingsLibraryKeyColorOffSubtitle =>
      'Rótulos de tom usam a cor de texto padrão em todo lugar.';

  @override
  String get settingsLibraryKeyColorAbsolute =>
      'Absoluto (círculo das quintas)';

  @override
  String get settingsLibraryKeyColorAbsoluteSubtitle =>
      'Cor fixa por tom na roda — maiores vivos, menores suaves (ex.: 8B forte, 8A mais suave).';

  @override
  String get settingsLibraryKeyColorHarmonic =>
      'Harmônico (deck em reprodução)';

  @override
  String get settingsLibraryKeyColorHarmonicSubtitle =>
      'Verde/amarelo em relação ao deck tocando — ex.: com 2A tocando, 1A/2A/3A/2B verdes, 1B/3B amarelos.';

  @override
  String get padStemsOffline => 'Offline';

  @override
  String get padStemsRealtime => 'Tempo real';

  @override
  String get settingsLibraryStemsTitle => 'Stems';

  @override
  String get settingsLibraryStemsDescription =>
      'Modelo e codec usados ao gravar caches de stems separados.';

  @override
  String get settingsLibraryStemFormat => 'Formato de stems';

  @override
  String get settingsLibraryStemModel => 'Modelo de stems';

  @override
  String get settingsLibraryStemOpus => 'Opus (160 kbps)';

  @override
  String get settingsLibraryStemFlac => 'FLAC (sem perda)';

  @override
  String get settingsLibraryDimPlayedTracks => 'Escurecer faixas já tocadas';

  @override
  String get settingsLibraryTrackRowLayout => 'Layout das linhas de faixa';

  @override
  String get settingsLibraryRowCompact => 'Compacto';

  @override
  String get settingsLibraryRowCompactSubtitle =>
      'Uma linha densa por faixa — cabe mais linhas na tela.';

  @override
  String get settingsLibraryRowComfortable => 'Confortável';

  @override
  String get settingsLibraryRowComfortableSubtitle =>
      'Duas linhas por faixa: título acima de artista, BPM, tom e duração.';

  @override
  String get settingsSessionDescription =>
      'Histórico de performance e limites de sessão.';

  @override
  String get settingsSessionHistoryTitle => 'Histórico de performance';

  @override
  String get settingsSessionHistoryDescription =>
      'Registra a reprodução dos decks em arquivos XSPF no suporte do app.';

  @override
  String get settingsSessionRecordHistory =>
      'Registrar histórico de performance';

  @override
  String get settingsSessionIdleTimeout => 'Tempo ocioso da sessão';

  @override
  String get settingsSessionIdleTimeoutHint =>
      'Fecha após esse tempo sem saída qualificada do deck.';

  @override
  String get settingsSessionMinPlayDuration => 'Duração mínima de reprodução';

  @override
  String get settingsSessionMinPlayDurationHint =>
      'Confirma entradas após essa quantidade de reprodução qualificada.';

  @override
  String get settingsSessionMinutes => 'minutos';

  @override
  String get settingsSessionSeconds => 'segundos';

  @override
  String get settingsSessionMinDeckVolume => 'Volume mínimo efetivo do deck';

  @override
  String settingsSessionPercentSemantics(int percent) {
    return '$percent por cento';
  }

  @override
  String get settingsStorageDescription =>
      'Uso de disco dos caches do Mixar e metadados da biblioteca no suporte do app.';

  @override
  String get settingsStorageOverviewTitle => 'Armazenamento do Mixar';

  @override
  String get settingsStorageOverviewDescription =>
      'Uso de disco dos caches e metadados da biblioteca no suporte do app.';

  @override
  String get settingsStorageMixarStorage => 'Armazenamento do Mixar';

  @override
  String settingsStorageUsed(String size) {
    return '$size usados';
  }

  @override
  String get settingsStorageStemCache => 'Cache de stems';

  @override
  String get settingsStorageStemModel => 'Modelo de stems';

  @override
  String get settingsStorageWaveform => 'Forma de onda';

  @override
  String get settingsStorageTrackMetadata => 'Metadados das faixas';

  @override
  String get settingsStorageSyncStemTitle => 'Sincronizar cache de stems?';

  @override
  String get settingsStorageSyncStemBody =>
      'Remove arquivos de stem que não estão no banco da biblioteca e apaga linhas do banco cujos arquivos faltam. Arquivos de cache referenciados e o áudio original são mantidos.';

  @override
  String get settingsStorageSyncFailed => 'Falha na sincronização';

  @override
  String get settingsStorageClearFailed => 'Falha ao limpar';

  @override
  String get settingsStorageClearStemTitle => 'Limpar cache de stems?';

  @override
  String get settingsStorageClearStemBody =>
      'Apaga arquivos de stem gerados de todas as faixas. O áudio original da biblioteca não é alterado.';

  @override
  String get settingsStorageClearStemOk => 'Cache de stems limpo';

  @override
  String get settingsStorageClearModelTitle =>
      'Limpar cache de modelos de stem?';

  @override
  String get settingsStorageClearModelBody =>
      'Apaga modelos de separação de stems baixados. Eles serão baixados de novo quando necessário.';

  @override
  String get settingsStorageClearModelOk => 'Modelo de stems limpo';

  @override
  String get settingsStorageClearWaveformTitle =>
      'Limpar cache de formas de onda?';

  @override
  String get settingsStorageClearWaveformBody =>
      'Apaga overviews de forma de onda em cache. Eles são regenerados ao abrir uma faixa.';

  @override
  String get settingsStorageClearWaveformOk => 'Cache de formas de onda limpo';

  @override
  String get settingsControllersDescription =>
      'Mapeamentos MIDI e hardware conectado. Confie em um dispositivo para ativá-lo automaticamente ao conectar depois de Salvar.';

  @override
  String get settingsControllersMappingsTitle => 'Mapeamentos';

  @override
  String get settingsControllersMappingsDescription =>
      'Armazenados nos dados do app. Mapas incluídos são copiados quando faltam; Atualizar sobrescreve a partir do pacote do app.';

  @override
  String get settingsControllersUpdateAll => 'Atualizar todos';

  @override
  String get settingsControllersNoMappings =>
      'Ainda não há mapeamentos nos dados do app.';

  @override
  String get settingsControllersMidiPortsTitle => 'Portas MIDI';

  @override
  String get settingsControllersMidiPortsDescription =>
      'Entradas e saídas MIDI detectadas e o mapeamento correspondente a cada uma.';

  @override
  String get settingsControllersNoMidiPorts => 'Nenhuma porta MIDI detectada.';

  @override
  String get settingsControllersNoMapping => 'Sem mapeamento';

  @override
  String settingsControllersMappingArrow(String mapping) {
    return '→ $mapping';
  }

  @override
  String get settingsControllersDirectionIn => 'IN';

  @override
  String get settingsControllersDirectionOut => 'OUT';

  @override
  String get settingsControllersAttached => 'Anexado';

  @override
  String get settingsControllersUpdateAvailable => 'Atualização disponível';

  @override
  String get settingsControllersTrust => 'Confiar';

  @override
  String get settingsControllersAttach => 'Anexar';

  @override
  String settingsControllersTrustSemantics(String name) {
    return 'Confiar no dispositivo $name';
  }

  @override
  String settingsControllersEnableSemantics(String name) {
    return 'Ativar $name';
  }

  @override
  String get mixxxImportTitle => 'Importar do Mixxx';

  @override
  String get mixxxImportDescription =>
      'Traga sua biblioteca Mixxx para o Mixar.';

  @override
  String mixxxImportFound(String path) {
    return 'Encontrado: $path';
  }

  @override
  String get mixxxImportCheckFailed =>
      'Não foi possível verificar uma biblioteca Mixxx.';

  @override
  String get mixxxImportLooking => 'Procurando uma biblioteca Mixxx…';

  @override
  String get mixxxImportNotFound =>
      'Nenhuma biblioteca Mixxx encontrada neste computador.';

  @override
  String get mixxxImportButton => 'Importar da biblioteca Mixxx…';

  @override
  String get mixxxImporting => 'Importando…';

  @override
  String get mixxxImportConfirmTitle => 'Importar do Mixxx?';

  @override
  String mixxxImportConfirmBody(
    String path,
    int trackCount,
    int missingFileCount,
    int playlistCount,
    int crateCount,
    int folderCount,
  ) {
    return 'Isso importa faixas, playlists, crates e pastas observadas de:\n$path\n\n$trackCount faixas ($missingFileCount ausentes), $playlistCount playlists, $crateCount crates, $folderCount pastas.\n\nArquivos ausentes são importados como faixas indisponíveis.';
  }

  @override
  String get mixxxImportReadFailed => 'Não foi possível ler a biblioteca Mixxx';

  @override
  String get mixxxImportFailed => 'Falha na importação do Mixxx';

  @override
  String mixxxImportFinishedWithErrors(int count) {
    return 'Importação do Mixxx terminou com $count erro(s)';
  }

  @override
  String get mixxxImportAlreadyImported => 'Biblioteca Mixxx já importada';

  @override
  String get mixxxImportNothing => 'Nada para importar do Mixxx';

  @override
  String mixxxImportUpdated(String updated, String missing) {
    return 'Atualizado do Mixxx: $updated$missing';
  }

  @override
  String mixxxImportImported(String parts, String missing) {
    return 'Importado $parts do Mixxx$missing';
  }

  @override
  String mixxxImportMissingSuffix(int count) {
    return ' ($count ausentes)';
  }

  @override
  String mixxxCountTracks(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count faixas',
      one: '1 faixa',
    );
    return '$_temp0';
  }

  @override
  String mixxxCountPlaylists(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count playlists',
      one: '1 playlist',
    );
    return '$_temp0';
  }

  @override
  String mixxxCountCrates(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count crates',
      one: '1 crate',
    );
    return '$_temp0';
  }

  @override
  String mixxxCountFolders(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pastas',
      one: '1 pasta',
    );
    return '$_temp0';
  }

  @override
  String mixxxCountUpdated(int count) {
    return '$count atualizadas';
  }

  @override
  String get rekordboxImportTitle => 'Importar do Rekordbox';

  @override
  String get rekordboxImportDescription =>
      'Traga sua biblioteca Rekordbox para o Mixar.';

  @override
  String rekordboxImportFound(String path) {
    return 'Encontrado: $path';
  }

  @override
  String get rekordboxImportCheckFailed =>
      'Não foi possível verificar uma biblioteca Rekordbox.';

  @override
  String get rekordboxImportLooking => 'Procurando uma biblioteca Rekordbox…';

  @override
  String get rekordboxImportNotFound =>
      'Nenhuma biblioteca Rekordbox encontrada neste computador.';

  @override
  String get rekordboxImportButton => 'Importar da biblioteca Rekordbox…';

  @override
  String get rekordboxImporting => 'Importando…';

  @override
  String get rekordboxImportConfirmTitle => 'Importar do Rekordbox?';

  @override
  String rekordboxImportConfirmBody(
    String path,
    int trackCount,
    int missingFileCount,
    int playlistCount,
    int smartPlaylistCount,
    int folderCount,
    int hotCueCount,
    int loopCount,
  ) {
    String _temp0 = intl.Intl.pluralLogic(
      trackCount,
      locale: localeName,
      other: '$trackCount faixas',
      one: '1 faixa',
    );
    String _temp1 = intl.Intl.pluralLogic(
      missingFileCount,
      locale: localeName,
      other: '$missingFileCount ausentes',
      one: '1 ausente',
    );
    String _temp2 = intl.Intl.pluralLogic(
      playlistCount,
      locale: localeName,
      other: '$playlistCount playlists',
      one: '1 playlist',
    );
    String _temp3 = intl.Intl.pluralLogic(
      smartPlaylistCount,
      locale: localeName,
      other: '$smartPlaylistCount inteligentes',
      one: '1 inteligente',
    );
    String _temp4 = intl.Intl.pluralLogic(
      folderCount,
      locale: localeName,
      other: '$folderCount pastas',
      one: '1 pasta',
    );
    String _temp5 = intl.Intl.pluralLogic(
      hotCueCount,
      locale: localeName,
      other: '$hotCueCount hot cues',
      one: '1 hot cue',
    );
    String _temp6 = intl.Intl.pluralLogic(
      loopCount,
      locale: localeName,
      other: '$loopCount loops',
      one: '1 loop',
    );
    return 'Isso importa faixas, playlists, hot cues e loops de:\n$path\n\n$_temp0 ($_temp1), $_temp2 ($_temp3), $_temp4, $_temp5, $_temp6.\n\nArquivos ausentes são importados como faixas indisponíveis.';
  }

  @override
  String get rekordboxImportReadFailed =>
      'Não foi possível ler a biblioteca Rekordbox';

  @override
  String get rekordboxImportFailed => 'Falha na importação do Rekordbox';

  @override
  String rekordboxImportFinishedWithErrors(int count) {
    return 'Importação do Rekordbox terminou com $count erro(s)';
  }

  @override
  String get rekordboxImportAlreadyImported =>
      'Biblioteca Rekordbox já importada';

  @override
  String get rekordboxImportNothing => 'Nada para importar do Rekordbox';

  @override
  String rekordboxImportUpdated(String updated, String missing) {
    return 'Atualizado do Rekordbox: $updated$missing';
  }

  @override
  String rekordboxImportImported(String parts, String missing) {
    return 'Importado $parts do Rekordbox$missing';
  }

  @override
  String rekordboxImportMissingSuffix(int count) {
    return ' ($count ausentes)';
  }

  @override
  String rekordboxCountTracks(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count faixas',
      one: '1 faixa',
    );
    return '$_temp0';
  }

  @override
  String rekordboxCountPlaylists(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count playlists',
      one: '1 playlist',
    );
    return '$_temp0';
  }

  @override
  String rekordboxCountHotCues(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count hot cues',
      one: '1 hot cue',
    );
    return '$_temp0';
  }

  @override
  String rekordboxCountLoops(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count loops',
      one: '1 loop',
    );
    return '$_temp0';
  }

  @override
  String rekordboxCountUpdated(int count) {
    return '$count atualizadas';
  }

  @override
  String get cdjExportTitle => 'Exportar para CDJ USB';

  @override
  String get cdjExportDescription =>
      'Grava playlists e coleções de pastas em um pendrive para CDJ/XDJ Pioneer, para os players navegarem sem o Rekordbox. Cada exportação é somada ao que o pendrive já tem.';

  @override
  String get cdjExportDestination => 'Destino';

  @override
  String get cdjExportDestinationHint => 'Pendrive ou pasta';

  @override
  String get cdjExportDestinationMissing =>
      'Escolha um destino (pendrive ou pasta).';

  @override
  String get cdjExportBrowse => 'Procurar…';

  @override
  String get cdjExportCollection => 'Coleção';

  @override
  String get cdjExportOverride => 'Sobrescrever arquivos já no pendrive';

  @override
  String get cdjExportConfirmAction => 'Exportar';

  @override
  String get cdjExporting => 'Exportando…';

  @override
  String get cdjExportFailed => 'Falha na exportação para CDJ USB';

  @override
  String cdjExportFinishedWithErrors(int count) {
    return 'Exportação para CDJ USB terminou com $count erro(s)';
  }

  @override
  String cdjExportFinishedWithWarnings(int count) {
    return 'Exportação para CDJ USB terminou com $count aviso(s)';
  }

  @override
  String cdjExportExported(String parts) {
    return 'Exportado $parts';
  }

  @override
  String get cdjExportNothing => 'Nada para exportar';

  @override
  String cdjCountTracks(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count faixas',
      one: '1 faixa',
    );
    return '$_temp0';
  }

  @override
  String cdjCountReused(int count) {
    return '$count reaproveitadas';
  }

  @override
  String cdjCountPlaylists(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count playlists',
      one: '1 playlist',
    );
    return '$_temp0';
  }

  @override
  String cdjCountFolders(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pastas',
      one: '1 pasta',
    );
    return '$_temp0';
  }

  @override
  String cdjCountCopied(String size) {
    return '$size copiados';
  }

  @override
  String get settingsStorageStemAlreadyInSync =>
      'Cache de stems já sincronizado';

  @override
  String settingsStorageRemovedOrphans(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Removidos $count arquivos de stem órfãos',
      one: 'Removido 1 arquivo de stem órfão',
    );
    return '$_temp0';
  }

  @override
  String get settingsAnalyticsTitle => 'Análises';

  @override
  String get settingsAnalyticsSubtitle =>
      'Partilhe dados anónimos da aplicação.';

  @override
  String get settingsAnalyticsToggle => 'Enviar dados anónimos de utilização';

  @override
  String get settingsLibraryVisibleFields => 'Campos visíveis';

  @override
  String get settingsLibraryFieldAlbum => 'Álbum';

  @override
  String get settingsLibraryFieldGenre => 'Género';

  @override
  String get settingsLibraryFieldYear => 'Ano';

  @override
  String get settingsLibraryFieldTrackNumber => 'Número da faixa';

  @override
  String get settingsLibraryFieldDiscNumber => 'Número do disco';

  @override
  String get settingsLibraryFieldComment => 'Comentário';

  @override
  String get settingsLibraryFieldIsrc => 'ISRC';

  @override
  String get settingsLibraryFieldDuration => 'Duração';

  @override
  String get settingsLibraryFieldRating => 'Avaliação';

  @override
  String get settingsLibraryFieldColor => 'Etiqueta de cor';

  @override
  String get settingsLibraryFieldBpm => 'BPM';

  @override
  String get settingsLibraryFieldKey => 'Tom';

  @override
  String onboardingLibraryNotFoundOne(String name) {
    return 'Nenhuma biblioteca do $name encontrada neste computador. Você pode importar uma depois em Configurações → Integrações.';
  }

  @override
  String get onboardingLibraryNotFoundBoth =>
      'Nenhuma biblioteca do Mixxx ou do Rekordbox encontrada neste computador. Você pode importar uma depois em Configurações → Integrações.';
}

/// The translations for Portuguese, as used in Brazil (`pt_BR`).
class AppLocalizationsPtBr extends AppLocalizationsPt {
  AppLocalizationsPtBr() : super('pt_BR');

  @override
  String get settingsPrivacyDescription =>
      'Dados anônimos de uso do aplicativo.';

  @override
  String get settingsLayoutPresetDescription =>
      'Comece com um dispositivo e depois mostre ou oculte cada painel.';

  @override
  String get settingsLayoutPresetMobile => 'Celular';

  @override
  String get settingsLayoutPresetMobileSubtitle =>
      'Decks com a forma de onda entre eles e a biblioteca em uma gaveta. Sem faixa do mixer.';

  @override
  String get settingsLayoutMixerMetersNoneSubtitle =>
      'Sem medidores nos decks; as colunas de canal dispensam a faixa do medidor.';

  @override
  String get settingsLayoutMixerMetersMonoSubtitle =>
      'Uma escada por deck (máximo de esquerda/direita).';

  @override
  String get settingsLayoutFxBarDescription =>
      'A unidade de efeitos compartilhada e o medidor master no fundo do painel do mixer. Os níveis master e de fones de ouvido abrem a partir do medidor.';

  @override
  String get settingsDeckVinylBrakePause => 'Freio de vinil no pause';

  @override
  String get settingsDeckVinylBrakePauseHint =>
      'Pause desacelera o deck como toca-discos, em vez de cortar na hora.';

  @override
  String get settingsDeckVinylBrakePauseToggle => 'Frear no pause';

  @override
  String get settingsDeckVinylBrakeStopTime => 'Tempo de parada';

  @override
  String get settingsDeckVinylBrakeStopTimeHint =>
      'Quanto tempo o prato leva para parar após o pause.';

  @override
  String settingsStepperDecrease(String field) {
    return 'Diminuir $field';
  }

  @override
  String settingsStepperIncrease(String field) {
    return 'Aumentar $field';
  }

  @override
  String get settingsLibraryStemsDescription =>
      'Modelo e codec usados ao gravar caches de stems separados.';

  @override
  String get settingsLibraryStemModel => 'Modelo de stems';

  @override
  String get settingsAnalyticsTitle => 'Análises';

  @override
  String get settingsAnalyticsSubtitle =>
      'Compartilhe dados anônimos do aplicativo.';

  @override
  String get settingsAnalyticsToggle => 'Enviar dados anônimos de uso';
}
