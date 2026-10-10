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
      'Salvar reinicia o engine automaticamente se ele estiver em execução.';

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
  String get settingsSectionLibrary => 'Biblioteca';

  @override
  String get settingsSectionStorage => 'Armazenamento';

  @override
  String get settingsSectionSession => 'Sessão';

  @override
  String get settingsSectionControllers => 'Controladores';

  @override
  String get settingsUiDescription =>
      'Chrome e dicas ao passar o mouse no app desktop.';

  @override
  String get settingsUiLanguageLabel => 'Idioma';

  @override
  String get settingsLanguageSystemDefault => 'Padrão do sistema';

  @override
  String get settingsShowTooltips => 'Mostrar dicas';

  @override
  String get settingsAudioDescription => 'Saída do engine e barramentos.';

  @override
  String get settingsAudioBackend => 'Backend';

  @override
  String get settingsAudioLowLatency => 'Baixa latência';

  @override
  String get settingsAudioSampleRate => 'Taxa de amostragem';

  @override
  String get settingsAudioSampleRateLoading =>
      'Carregando taxas do dispositivo de saída master…';

  @override
  String get settingsAudioResamplerQuality => 'Qualidade do resampler';

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
  String get settingsAudioPreviewBus => 'Barramento de preview (fones / cue)';

  @override
  String get settingsAudioDevice => 'Dispositivo';

  @override
  String get settingsAudioChannelMode => 'Modo de canal';

  @override
  String get settingsAudioStereoPair => 'Par estéreo';

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
      'Mantém faixas analisadas com volume percebido consistente.';

  @override
  String get settingsMixerVolumeNormalizer => 'Normalizador de volume';

  @override
  String get settingsMixerTargetLufs => 'LUFS alvo';

  @override
  String get settingsWaveformDescription =>
      'RGB mistura grave/médio/agudo em uma cor. Filtrado empilha as três bandas.';

  @override
  String get settingsWaveformDisplayMode => 'Modo de exibição';

  @override
  String get settingsWaveformModeRgb => 'RGB';

  @override
  String get settingsWaveformModeFiltered => 'Filtrado';

  @override
  String get settingsDeckDescription =>
      'Comportamento padrão de jog, tempo e sampler para novos decks.';

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
  String get settingsDeckSamplerStripRoute => 'Rota do sampler no strip';

  @override
  String get settingsDeckSamplerStripBefore => 'Antes do channel strip';

  @override
  String get settingsDeckSamplerStripAfter => 'Depois do channel strip';

  @override
  String get settingsDeckDefaultSamplerBankA =>
      'Banco padrão do sampler no deck A';

  @override
  String get settingsDeckDefaultSamplerBankB =>
      'Banco padrão do sampler no deck B';

  @override
  String get settingsLibraryDescription =>
      'Importação de faixas, análise offline e exibição da lista.';

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
  String get settingsLibraryStemFormat => 'Formato de stems';

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
      'Armazenados nos dados do app. A semente copia mapas empacotados quando faltam; Atualizar sobrescreve a partir do pacote do app.';

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
      'Traga faixas, playlists e crates da sua biblioteca Mixxx para o Mixar.';

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
    return 'Atualizado $updated do Mixxx$missing';
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
}

/// The translations for Portuguese, as used in Brazil (`pt_BR`).
class AppLocalizationsPtBr extends AppLocalizationsPt {
  AppLocalizationsPtBr() : super('pt_BR');

  @override
  String get settingsTitle => 'CONFIGURAÇÕES';

  @override
  String get settingsSubtitle =>
      'Salvar reinicia o engine automaticamente se ele estiver em execução.';

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
  String get settingsSectionLibrary => 'Biblioteca';

  @override
  String get settingsSectionStorage => 'Armazenamento';

  @override
  String get settingsSectionSession => 'Sessão';

  @override
  String get settingsSectionControllers => 'Controladores';

  @override
  String get settingsUiDescription =>
      'Chrome e dicas ao passar o mouse no app desktop.';

  @override
  String get settingsUiLanguageLabel => 'Idioma';

  @override
  String get settingsLanguageSystemDefault => 'Padrão do sistema';

  @override
  String get settingsShowTooltips => 'Mostrar dicas';

  @override
  String get settingsAudioDescription => 'Saída do engine e barramentos.';

  @override
  String get settingsAudioBackend => 'Backend';

  @override
  String get settingsAudioLowLatency => 'Baixa latência';

  @override
  String get settingsAudioSampleRate => 'Taxa de amostragem';

  @override
  String get settingsAudioSampleRateLoading =>
      'Carregando taxas do dispositivo de saída master…';

  @override
  String get settingsAudioResamplerQuality => 'Qualidade do resampler';

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
  String get settingsAudioPreviewBus => 'Barramento de preview (fones / cue)';

  @override
  String get settingsAudioDevice => 'Dispositivo';

  @override
  String get settingsAudioChannelMode => 'Modo de canal';

  @override
  String get settingsAudioStereoPair => 'Par estéreo';

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
      'Mantém faixas analisadas com volume percebido consistente.';

  @override
  String get settingsMixerVolumeNormalizer => 'Normalizador de volume';

  @override
  String get settingsMixerTargetLufs => 'LUFS alvo';

  @override
  String get settingsWaveformDescription =>
      'RGB mistura grave/médio/agudo em uma cor. Filtrado empilha as três bandas.';

  @override
  String get settingsWaveformDisplayMode => 'Modo de exibição';

  @override
  String get settingsWaveformModeRgb => 'RGB';

  @override
  String get settingsWaveformModeFiltered => 'Filtrado';

  @override
  String get settingsDeckDescription =>
      'Comportamento padrão de jog, tempo e sampler para novos decks.';

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
  String get settingsDeckSamplerStripRoute => 'Rota do sampler no strip';

  @override
  String get settingsDeckSamplerStripBefore => 'Antes do channel strip';

  @override
  String get settingsDeckSamplerStripAfter => 'Depois do channel strip';

  @override
  String get settingsDeckDefaultSamplerBankA =>
      'Banco padrão do sampler no deck A';

  @override
  String get settingsDeckDefaultSamplerBankB =>
      'Banco padrão do sampler no deck B';

  @override
  String get settingsLibraryDescription =>
      'Importação de faixas, análise offline e exibição da lista.';

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
  String get settingsLibraryStemFormat => 'Formato de stems';

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
      'Armazenados nos dados do app. A semente copia mapas empacotados quando faltam; Atualizar sobrescreve a partir do pacote do app.';

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
      'Traga faixas, playlists e crates da sua biblioteca Mixxx para o Mixar.';

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
    return 'Atualizado $updated do Mixxx$missing';
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
}
