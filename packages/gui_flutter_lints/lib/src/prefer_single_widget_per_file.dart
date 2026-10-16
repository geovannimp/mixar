import 'package:analyzer/analysis_rule/analysis_rule.dart';
import 'package:analyzer/analysis_rule/rule_context.dart';
import 'package:analyzer/analysis_rule/rule_visitor_registry.dart';
import 'package:analyzer/dart/ast/ast.dart';
import 'package:analyzer/dart/ast/visitor.dart';
import 'package:analyzer/dart/element/type.dart';
import 'package:analyzer/error/error.dart';

/// Widget base class names used when type resolution is unavailable.
///
/// Covers the widget roots used across `apps/gui-flutter` (Flutter, Riverpod
/// and `flutter_hooks` variants). Generic arguments are ignored.
const _widgetBaseNames = <String>{
  'StatelessWidget',
  'StatefulWidget',
  'ConsumerWidget',
  'ConsumerStatefulWidget',
  'InheritedWidget',
  'HookWidget',
  'HookConsumerWidget',
};

/// `State` / `ConsumerState` subclasses are companions of their widget and are
/// not counted as widgets themselves.
const _stateBaseNames = <String>{'State', 'ConsumerState'};

/// Flags files declaring more than one public `Widget` subclass.
class PreferSingleWidgetPerFile extends AnalysisRule {
  PreferSingleWidgetPerFile()
    : super(name: code.lowerCaseName, description: code.problemMessage);

  static const code = LintCode(
    'prefer_single_widget_per_file',
    'Only one public Widget subclass is allowed per file. Found: {0}.',
    correctionMessage: 'Split every public Widget subclass into its own file.',
    severity: DiagnosticSeverity.WARNING,
  );

  @override
  DiagnosticCode get diagnosticCode => code;

  @override
  void registerNodeProcessors(
    RuleVisitorRegistry registry,
    RuleContext context,
  ) {
    registry.addCompilationUnit(this, _Visitor(this, context));
  }
}

class _Visitor extends SimpleAstVisitor<void> {
  _Visitor(this.rule, this.context);

  final PreferSingleWidgetPerFile rule;
  final RuleContext context;

  @override
  void visitCompilationUnit(CompilationUnit node) {
    final path = context.currentUnit?.file.path ?? '';
    // Defense in depth: generated FRB code is analyzer-excluded already.
    if (path.contains('/src/rust/')) return;
    if (path.endsWith('.g.dart')) return;

    final widgetClasses = <ClassDeclaration>[];
    for (final declaration in node.declarations) {
      if (declaration is! ClassDeclaration) continue;
      if (!_isPublic(declaration)) continue;
      if (_isStateSubclass(declaration)) continue;
      if (!_isWidgetSubclass(declaration)) continue;
      widgetClasses.add(declaration);
    }

    if (widgetClasses.length < 2) return;

    final names = widgetClasses
        .map((c) => c.namePart.typeName.lexeme)
        .join(', ');
    for (final declaration in widgetClasses) {
      rule.reportAtNode(declaration, arguments: [names]);
    }
  }

  /// Public = the identifier does not start with `_`.
  bool _isPublic(ClassDeclaration node) =>
      !node.namePart.typeName.lexeme.startsWith('_');

  bool _isStateSubclass(ClassDeclaration node) {
    if (!node.namePart.typeName.lexeme.endsWith('State')) return false;
    if (_hasNamedBase(node, _stateBaseNames)) return true;
    final supertype = node.declaredFragment?.element.supertype;
    return supertype != null &&
        _supertypes(supertype)
            .any((type) => _stateBaseNames.contains(type.element.name));
  }

  /// A widget is either resolvable to a `Widget` subtype, or — when the element
  /// model is unresolved — names one of the known widget base classes in its
  /// `extends` clause.
  bool _isWidgetSubclass(ClassDeclaration node) {
    final supertype = node.declaredFragment?.element.supertype;
    if (supertype != null && _hasWidgetAncestor(supertype)) return true;
    // Name-based fallback for unresolved units.
    return _hasNamedBase(node, _widgetBaseNames);
  }

  bool _hasNamedBase(ClassDeclaration node, Set<String> names) =>
      _namedBases(node).any(names.contains);

  Iterable<String> _namedBases(ClassDeclaration node) sync* {
    final extendsClause = node.extendsClause;
    if (extendsClause != null) {
      yield extendsClause.superclass.name.lexeme;
    }
    final withClause = node.withClause;
    if (withClause != null) {
      for (final type in withClause.mixinTypes) {
        yield type.name.lexeme;
      }
    }
  }

  /// Walks the full supertype chain of [type], looking for `Widget`.
  bool _hasWidgetAncestor(InterfaceType type) => _supertypes(type).any(
    (supertype) =>
        supertype.element.name == 'Widget' &&
        supertype.element.library.name == 'widgets',
  );

  Iterable<InterfaceType> _supertypes(InterfaceType type) sync* {
    final seen = <String>{};
    final queue = <InterfaceType>[type];
    while (queue.isNotEmpty) {
      final current = queue.removeLast();
      final uri = current.element.library.uri.toString();
      final name = current.element.name ?? '';
      final key = '$uri::$name';
      if (!seen.add(key)) continue;
      yield current;
      queue.addAll(current.allSupertypes);
    }
  }
}
