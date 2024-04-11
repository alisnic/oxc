use oxc_ast::{
    ast::{ModuleDeclaration, TSInterfaceDeclaration, TSTypeName},
    AstKind,
};
use oxc_diagnostics::{
    miette::{self, Diagnostic},
    thiserror::{self, Error},
};
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;

use crate::{context::LintContext, rule::Rule, AstNode};

#[derive(Debug, Error, Diagnostic)]
#[error("typescript-eslint(no-unused-vars): test")]
#[diagnostic(severity(warning), help("test"))]
struct NoUnusedVarsDiagnostic(#[label] pub Span);

#[derive(Debug, Default, Clone)]
pub struct NoUnusedVars;

declare_oxc_lint!(
    /// ### What it does
    ///
    ///
    /// ### Why is this bad?
    ///
    ///
    /// ### Example
    /// ```javascript
    /// ```
    NoUnusedVars,
    pedantic
);

impl Rule for NoUnusedVars {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        let symbols = ctx.semantic().symbols();
        let nodes = ctx.semantic().nodes();
        dbg!(node);

        match node.kind() {
            oxc_ast::AstKind::BindingIdentifier(ident) => {
                let Some(symbol_id) = ident.symbol_id.get() else {
                    return;
                };

                let references = symbols.get_resolved_reference_ids(symbol_id);
                if !references.is_empty() {
                    return;
                }

                if let Some(interface) = find_parent_interface(node, ctx) {
                    // TODO: interface implementations are not listed in get_resolved_reference_ids
                    println!("HERE {:?}", interface);
                    if interface_has_implementations(ctx, &interface.id.name) {
                        return;
                    }
                }

                let is_exported = nodes.iter_parents(node.id()).any(|parent| {
                    matches!(
                        parent.kind(),
                        AstKind::ModuleDeclaration(ModuleDeclaration::ExportNamedDeclaration(_))
                    )
                });

                if is_exported {
                    return;
                };

                ctx.diagnostic(NoUnusedVarsDiagnostic(ident.span));
                // dbg!(references);
            }
            _ => {}
        }
    }
}

fn interface_has_implementations<'a>(ctx: &LintContext<'a>, name: &oxc_span::Atom<'a>) -> bool {
    ctx.nodes().iter().any(|node| match node.kind() {
        AstKind::Class(class) => {
            let Some(impls) = &class.implements else {
                return false;
            };

            dbg!(impls);

            impls.iter().any(|implementation| {
                let TSTypeName::IdentifierReference(iref) = &implementation.expression else {
                    return false;
                };

                println!("{:?} {:?}", iref.name, name);
                return iref.name == name;
            })
        }
        _ => false,
    })
}

fn find_parent_interface<'a>(
    node: &AstNode<'a>,
    ctx: &LintContext<'a>,
) -> Option<&'a TSInterfaceDeclaration<'a>> {
    ctx.nodes().iter_parents(node.id()).find_map(|parent| match parent.kind() {
        AstKind::TSInterfaceDeclaration(iface) => Some(iface),
        _ => None,
    })
}

#[test]
fn test() {
    use crate::tester::Tester;

    let pass = vec![
        r"import { ClassDecoratorFactory } from 'decorators';
        @ClassDecoratorFactory()
        export class Foo {}",
        r"import { ClassDecorator } from 'decorators';
        @ClassDecorator
        export class Foo {}",
        r"import { AccessorDecoratorFactory } from 'decorators';
        export class Foo {
          @AccessorDecoratorFactory(true)
          get bar() {}
        }",
        r"import { AccessorDecorator } from 'decorators';
        export class Foo {
          @AccessorDecorator
          set bar() {}
        }",
        r"import { MethodDecoratorFactory } from 'decorators';
        export class Foo {
          @MethodDecoratorFactory(false)
          bar() {}
        }",
        r"import { MethodDecorator } from 'decorators';
        export class Foo {
          @MethodDecorator
          static bar() {}
        }",
        r"import { ConstructorParameterDecoratorFactory } from 'decorators';
        export class Service {
          constructor(
            @ConstructorParameterDecoratorFactory(APP_CONFIG) config: AppConfig,
          ) {
            this.title = config.title;
          }
        }",
        r"import { ConstructorParameterDecorator } from 'decorators';
        export class Foo {
          constructor(@ConstructorParameterDecorator bar) {
            this.bar = bar;
          }
        }",
        r"import { ParameterDecoratorFactory } from 'decorators';
        export class Qux {
          bar(@ParameterDecoratorFactory(true) baz: number) {
            console.log(baz);
          }
        }",
        r"import { ParameterDecorator } from 'decorators';
        export class Foo {
          static greet(@ParameterDecorator name: string) {
            return name;
          }
        }",
        r"import { Input, Output, EventEmitter } from 'decorators';
        export class SomeComponent {
          @Input() data;
          @Output()
          click = new EventEmitter();
        }",
        r"import { configurable } from 'decorators';
        export class A {
          @configurable(true) static prop1;

          @configurable(false)
          static prop2;
        }",
        r"import { foo, bar } from 'decorators';
        export class B {
          @foo x;

          @bar
          y;
        }",
        r"interface Base {}
        class Thing implements Base {}
        new Thing();",
        r"interface Base {}
        const a: Base = {};
        console.log(a);",
        r"import { Foo } from 'foo';
        function bar<T>(): T {}
        bar<Foo>();",
        r"import { Foo } from 'foo';
        const bar = function <T>(): T {};
        bar<Foo>();",
        // r"import { Foo } from 'foo';
        // const bar = <T,>(): T => {};
        // bar<Foo>();",
        // r"import { Foo } from 'foo';
        // <Foo>(<T,>(): T => {})();",
        r"import { Nullable } from 'nullable';
        const a: Nullable<string> = 'hello';
        console.log(a);",
        r"import { Nullable } from 'nullable';
        import { SomeOther } from 'other';
        const a: Nullable<SomeOther> = 'hello';
        console.log(a);",
        r"import { Nullable } from 'nullable';
        const a: Nullable | undefined = 'hello';
        console.log(a);",
        r"import { Nullable } from 'nullable';
        const a: Nullable & undefined = 'hello';
        console.log(a);",
        r"import { Nullable } from 'nullable';
        import { SomeOther } from 'other';
        const a: Nullable<SomeOther[]> = 'hello';
        console.log(a);",
        r"import { Nullable } from 'nullable';
        import { SomeOther } from 'other';
        const a: Nullable<Array<SomeOther>> = 'hello';
        console.log(a);",
        r"import { Nullable } from 'nullable';
        const a: Array<Nullable> = 'hello';
        console.log(a);",
        r"import { Nullable } from 'nullable';
        const a: Nullable[] = 'hello';
        console.log(a);",
        r"import { Nullable } from 'nullable';
        const a: Array<Nullable[]> = 'hello';
        console.log(a);",
        r"import { Nullable } from 'nullable';
        const a: Array<Array<Nullable>> = 'hello';
        console.log(a);",
        r"import { Nullable } from 'nullable';
        import { SomeOther } from 'other';
        const a: Array<Nullable<SomeOther>> = 'hello';
        console.log(a);",
        r"import { Nullable } from 'nullable';
        import { Component } from 'react';
        class Foo implements Component<Nullable> {}

        new Foo();",
        r"import { Nullable } from 'nullable';
        import { Component } from 'react';
        class Foo extends Component<Nullable, {}> {}
        new Foo();",
        r"import { Nullable } from 'nullable';
        import { SomeOther } from 'some';
        import { Component } from 'react';
        class Foo extends Component<Nullable<SomeOther>, {}> {}
        new Foo();",
        r"import { Nullable } from 'nullable';
        import { SomeOther } from 'some';
        import { Component } from 'react';
        class Foo implements Component<Nullable<SomeOther>, {}> {}
        new Foo();",
        r"import { Nullable } from 'nullable';
        import { SomeOther } from 'some';
        import { Component, Component2 } from 'react';
        class Foo implements Component<Nullable<SomeOther>, {}>, Component2 {}
        new Foo();",
        r"import { Nullable } from 'nullable';
        import { Another } from 'some';
        class A {
          do = (a: Nullable<Another>) => {
            console.log(a);
          };
        }
        new A();",
        r"import { Nullable } from 'nullable';
        import { Another } from 'some';
        class A {
          do(a: Nullable<Another>) {
            console.log(a);
          }
        }
        new A();",
        r"import { Nullable } from 'nullable';
        import { Another } from 'some';
        class A {
          do(): Nullable<Another> {
            return null;
          }
        }
        new A();",
        r"import { Nullable } from 'nullable';
        import { Another } from 'some';
        export interface A {
          do(a: Nullable<Another>);
        }",
        r"import { Nullable } from 'nullable';
        import { Another } from 'some';
        export interface A {
          other: Nullable<Another>;
        }",
        r"import { Nullable } from 'nullable';
        function foo(a: Nullable) {
          console.log(a);
        }
        foo();",
        r"import { Nullable } from 'nullable';
        function foo(): Nullable {
          return null;
        }
        foo();",
        r"import { Nullable } from 'nullable';
        import { SomeOther } from 'some';
        class A extends Nullable<SomeOther> {
          other: Nullable<Another>;
        }
        new A();",
        r"import { Nullable } from 'nullable';
        import { SomeOther } from 'some';
        import { Another } from 'some';
        class A extends Nullable<SomeOther> {
          do(a: Nullable<Another>) {
            console.log(a);
          }
        }
        new A();",
        r"import { Nullable } from 'nullable';
        import { SomeOther } from 'some';
        import { Another } from 'some';
        export interface A extends Nullable<SomeOther> {
          other: Nullable<Another>;
        }",
        r"import { Nullable } from 'nullable';
        import { SomeOther } from 'some';
        import { Another } from 'some';
        export interface A extends Nullable<SomeOther> {
          do(a: Nullable<Another>);
        }",
        r"import { Foo } from './types';

        class Bar<T extends Foo> {
          prop: T;
        }

        new Bar<number>();",
        r"import { Foo, Bar } from './types';

        class Baz<T extends Foo & Bar> {
          prop: T;
        }

        new Baz<any>();",
        r"import { Foo } from './types';

        class Bar<T = Foo> {
          prop: T;
        }

        new Bar<number>();",
        r"import { Foo } from './types';

        class Foo<T = any> {
          prop: T;
        }

        new Foo();",
        r"import { Foo } from './types';

        class Foo<T = {}> {
          prop: T;
        }

        new Foo();",
        r"import { Foo } from './types';

        class Foo<T extends {} = {}> {
          prop: T;
        }

        new Foo();",
        r"type Foo = 'a' | 'b' | 'c';
        type Bar = number;

        export const map: { [name in Foo]: Bar } = {
          a: 1,
          b: 2,
          c: 3,
        };",
        r"type Foo = 'a' | 'b' | 'c';
        type Bar = number;

        export const map: { [name in Foo as string]: Bar } = {
          a: 1,
          b: 2,
          c: 3,
        };",
        r"import { Nullable } from 'nullable';
        class A<T> {
          bar: T;
        }
        new A<Nullable>();",
        r"import { Nullable } from 'nullable';
        import { SomeOther } from 'other';
        function foo<T extends Nullable>(): T {}
        foo<SomeOther>();",
        r"import { Nullable } from 'nullable';
        import { SomeOther } from 'other';
        class A<T extends Nullable> {
          bar: T;
        }
        new A<SomeOther>();",
        r"import { Nullable } from 'nullable';
        import { SomeOther } from 'other';
        interface A<T extends Nullable> {
          bar: T;
        }
        export const a: A<SomeOther> = {
          foo: 'bar',
        };",
        r"export class App {
          constructor(private logger: Logger) {
            console.log(this.logger);
          }
        }",
        r"export class App {
          constructor(bar: string);
          constructor(private logger: Logger) {
            console.log(this.logger);
          }
        }",
        r"export class App {
          constructor(
            baz: string,
            private logger: Logger,
          ) {
            console.log(baz);
            console.log(this.logger);
          }
        }",
        r"export class App {
          constructor(
            baz: string,
            private logger: Logger,
            private bar: () => void,
          ) {
            console.log(this.logger);
            this.bar();
          }
        }",
        r"export class App {
          constructor(private logger: Logger) {}
          meth() {
            console.log(this.logger);
          }
        }",
        r"import { Component, Vue } from 'vue-property-decorator';
        import HelloWorld from './components/HelloWorld.vue';

        @Component({
          components: {
            HelloWorld,
          },
        })
        export default class App extends Vue {}",
        r"import firebase, { User } from 'firebase/app';
        // initialize firebase project
        firebase.initializeApp({});
        export function authenticated(cb: (user: User | null) => void): void {
          firebase.auth().onAuthStateChanged(user => cb(user));
        }",
        r"import { Foo } from './types';
        export class Bar<T extends Foo> {
          prop: T;
        }",
        r"import webpack from 'webpack';
        export default function webpackLoader(this: webpack.loader.LoaderContext) {}",
        r"import execa, { Options as ExecaOptions } from 'execa';
        export function foo(options: ExecaOptions): execa {
          options();
        }",
        r"import { Foo, Bar } from './types';
        export class Baz<F = Foo & Bar> {
          prop: F;
        }",
        r"// warning 'B' is defined but never used
        export const a: Array<{ b: B }> = [];",
        r"export enum FormFieldIds {
          PHONE = 'phone',
          EMAIL = 'email',
        }",
        r"enum FormFieldIds {
          PHONE = 'phone',
          EMAIL = 'email',
        }
        export interface IFoo {
          fieldName: FormFieldIds;
        }",
        r"enum FormFieldIds {
          PHONE = 'phone',
          EMAIL = 'email',
        }
        export interface IFoo {
          fieldName: FormFieldIds.EMAIL;
        }",
        r"import * as fastify from 'fastify';
        import { Server, IncomingMessage, ServerResponse } from 'http';
        const server: fastify.FastifyInstance<Server, IncomingMessage, ServerResponse> =
          fastify({});
        server.get('/ping');",
        r"declare namespace Foo {
          function bar(line: string, index: number | null, tabSize: number): number;
          var baz: string;
        }
        console.log(Foo);",
        r"import foo from 'foo';
        export interface Bar extends foo.i18n {}",
        r"import foo from 'foo';
        import bar from 'foo';
        export interface Bar extends foo.i18n<bar> {}",
        r"import { TypeA } from './interface';
        export const a = <GenericComponent<TypeA> />;",
        r"const text = 'text';
        export function Foo() {
          return (
            <div>
              <input type='search' size={30} placeholder={text} />
            </div>
          );
        }",
        r"import { observable } from 'mobx';
        export default class ListModalStore {
          @observable
          orderList: IObservableArray<BizPurchaseOrderTO> = observable([]);
        }",
        r"import { Dec, TypeA, Class } from 'test';
        export default class Foo {
          constructor(
            @Dec(Class)
            private readonly prop: TypeA<Class>,
          ) {}
        }",
        r"import { Dec, TypeA, Class } from 'test';
        export default class Foo {
          constructor(
            @Dec(Class)
            ...prop: TypeA<Class>
          ) {
            prop();
          }
        }",
        r"export function foo(): void;
        export function foo(): void;
        export function foo(): void {}",
        r"export function foo(a: number): number;
        export function foo(a: string): string;
        export function foo(a: number | string): number | string {
          return a;
        }",
        r"export function foo<T>(a: number): T;
        export function foo<T>(a: string): T;
        export function foo<T>(a: number | string): T {
          return a;
        }",
        r"export type T = {
          new (): T;
          new (arg: number): T;
          new <T>(arg: number): T;
        };",
        // r"export type T = new () => T;
        // export type T = new (arg: number) => T;
        // export type T = new <T>(arg: number) => T;",
        r"enum Foo {
          a,
        }
        export type T = {
          [Foo.a]: 1;
        };",
        r"namespace Foo {
          export const Foo = 1;
        }

        export { Foo };",
        r"export namespace Foo {
          export const item: Foo = 1;
        }",
        r"namespace foo.bar {
          export interface User {
            name: string;
          }
        }",
        r"export interface Foo {
          bar: string;
          baz: Foo['bar'];
        }",
        r"export type Bar = Array<Bar>;",
        r"function Foo() {}

        namespace Foo {
          export const x = 1;
        }

        export { Foo };",
        r"class Foo {}

        namespace Foo {
          export const x = 1;
        }

        export { Foo };",
        r"namespace Foo {}

        const Foo = 1;

        export { Foo };",
        r"type Foo = {
          error: Error | null;
        };

        export function foo() {
          return new Promise<Foo>();
        }",
        r"function foo<T>(value: T): T {
          return { value };
        }
        export type Foo<T> = typeof foo<T>;",
        r"export interface Event<T> {
          (
            listener: (e: T) => any,
            thisArgs?: any,
            disposables?: Disposable[],
          ): Disposable;
        }",
        r"export class Test {
          constructor(@Optional() value: number[] = []) {
            console.log(value);
          }
        }

        function Optional() {
          return () => {};
        }",
        r"import { FooType } from './fileA';

        export abstract class Foo {
          protected abstract readonly type: FooType;
        }",
        r"export type F<A extends unknown[]> = (...a: A) => unknown;",
        r"import { Foo } from './bar';
        export type F<A extends unknown[]> = (...a: Foo<A>) => unknown;",
        r"type StyledPaymentProps = {
          isValid: boolean;
        };

        export const StyledPayment = styled.div<StyledPaymentProps>``;",
        r"import type { foo } from './a';
        export type Bar = typeof foo;",
        r"interface Foo {}
        type Bar = {};
        declare class Clazz {}
        declare function func();
        declare enum Enum {}
        declare namespace Name {}
        declare const v1 = 1;
        declare var v2 = 1;
        declare let v3 = 1;
        declare const { v4 };
        declare const { v4: v5 };
        declare const [v6];",
        r"export type Test<U> = U extends (k: infer I) => void ? I : never;",
        r"export type Test<U> = U extends { [k: string]: infer I } ? I : never;",
        r"export type Test<U> = U extends (arg: {
          [k: string]: (arg2: infer I) => void;
        }) => void
          ? I
          : never;",
        r"import React from 'react';

                export const ComponentFoo: React.FC = () => {
                  return <div>Foo Foo</div>;
                };",
        r"import { h } from 'some-other-jsx-lib';

                export const ComponentFoo: h.FC = () => {
                  return <div>Foo Foo</div>;
                };",
        r"import { Fragment } from 'react';

                export const ComponentFoo: Fragment = () => {
                  return <>Foo Foo</>;
                };",
        r"declare module 'foo' {
          type Test = 1;
        }",
        r"declare module 'foo' {
          type Test = 1;
          const x: Test = 1;
          export = x;
        }",
        r"declare global {
          interface Foo {}
        }",
        r"declare global {
          namespace jest {
            interface Matchers<R> {
              toBeSeven: () => R;
            }
          }
        }",
        r"export declare namespace Foo {
          namespace Bar {
            namespace Baz {
              namespace Bam {
                const x = 1;
              }
            }
          }
        }",
        r"class Foo<T> {
          value: T;
        }
        class Bar<T> {
          foo = Foo<T>;
        }
        new Bar();",
        r"declare namespace A {
          export interface A {}
        }",
        r"declare function A(A: string): string;",
        r"type Color = 'red' | 'blue';
        type Quantity = 'one' | 'two';
        export type SeussFish = `${Quantity | Color} fish`;",
        r"type VerticalAlignment = 'top' | 'middle' | 'bottom';
        type HorizontalAlignment = 'left' | 'center' | 'right';

        export declare function setAlignment(value: `${VerticalAlignment}-${HorizontalAlignment}`): void;",
        r"type EnthusiasticGreeting<T extends string> = `${Uppercase<T>} - ${Lowercase<T>} - ${Capitalize<T>} - ${Uncapitalize<T>}`;
        export type HELLO = EnthusiasticGreeting<'heLLo'>;",
        r"interface IItem {
          title: string;
          url: string;
          children?: IItem[];
        }",
        r"namespace _Foo {
          export const bar = 1;
          export const baz = Foo.bar;
        }",
        r"interface _Foo {
          a: string;
          b: Foo;
        }",
        r"/* eslint collect-unused-vars: 'error' */
        declare module 'next-auth' {
          interface User {
            id: string;
            givenName: string;
            familyName: string;
          }
        }",
        r"import { TestGeneric, Test } from 'fake-module';

        declare function deco(..._param: any): any;
        export class TestClass {
          @deco
          public test(): TestGeneric<Test> {}
        }",
        r"function foo() {}

        export class Foo {
          constructor() {
            foo();
          }
        }",
        r"function foo() {}

        export class Foo {
          static {}

          constructor() {
            foo();
          }
        }",
        r"interface Foo {
          bar: string;
        }
        export const Foo = 'bar';",
        r"export const Foo = 'bar';
        interface Foo {
          bar: string;
        }",
        r"let foo = 1;
        foo ??= 2;",
        r"let foo = 1;
        foo &&= 2;",
        r"let foo = 1;
        foo ||= 2;",
        r"const foo = 1;
        export = foo;",
        r"const Foo = 1;
        interface Foo {
          bar: string;
        }
        export = Foo;",
        r"interface Foo {
          bar: string;
        }
        export = Foo;",
        r"type Foo = 1;
        export = Foo;",
        r"type Foo = 1;
        export = {} as Foo;",
        r"declare module 'foo' {
          type Foo = 1;
          export = Foo;
        }",
        r"namespace Foo {
          export const foo = 1;
        }
        export namespace Bar {
          export import TheFoo = Foo;
        }",
    ];

    let fail = vec![
        r"import { ClassDecoratorFactory } from 'decorators';
        export class Foo {}",
        r"import { Foo, Bar } from 'foo';
        function baz<Foo>(): Foo {}
        baz<Bar>();",
        r"import { Nullable } from 'nullable';
        const a: string = 'hello';
        console.log(a);",
        r"import { Nullable } from 'nullable';
        import { SomeOther } from 'other';
        const a: Nullable<string> = 'hello';
        console.log(a);",
        r"import { Nullable } from 'nullable';
        import { Another } from 'some';
        class A {
          do = (a: Nullable) => {
            console.log(a);
          };
        }
        new A();",
        r"import { Nullable } from 'nullable';
        import { Another } from 'some';
        class A {
          do(a: Nullable) {
            console.log(a);
          }
        }
        new A();",
        r"import { Nullable } from 'nullable';
        import { Another } from 'some';
        class A {
          do(): Nullable {
            return null;
          }
        }
        new A();",
        r"import { Nullable } from 'nullable';
        import { Another } from 'some';
        export interface A {
          do(a: Nullable);
        }",
        r"import { Nullable } from 'nullable';
        import { Another } from 'some';
        export interface A {
          other: Nullable;
        }",
        r"import { Nullable } from 'nullable';
        function foo(a: string) {
          console.log(a);
        }
        foo();",
        r"import { Nullable } from 'nullable';
        function foo(): string | null {
          return null;
        }
        foo();",
        r"import { Nullable } from 'nullable';
        import { SomeOther } from 'some';
        import { Another } from 'some';
        class A extends Nullable {
          other: Nullable<Another>;
        }
        new A();",
        r"import { Nullable } from 'nullable';
        import { SomeOther } from 'some';
        import { Another } from 'some';
        abstract class A extends Nullable {
          other: Nullable<Another>;
        }
        new A();",
        r"enum FormFieldIds {
          PHONE = 'phone',
          EMAIL = 'email',
        }",
        r"import test from 'test';
        import baz from 'baz';
        export interface Bar extends baz.test {}",
        r"import test from 'test';
        import baz from 'baz';
        export interface Bar extends baz().test {}",
        r"import test from 'test';
        import baz from 'baz';
        export class Bar implements baz.test {}",
        r"import test from 'test';
        import baz from 'baz';
        export class Bar implements baz().test {}",
        r"namespace Foo {}",
        r"namespace Foo {
          export const Foo = 1;
        }",
        r"namespace Foo {
          const Foo = 1;
          console.log(Foo);
        }",
        r"namespace Foo {
          export const Bar = 1;
          console.log(Foo.Bar);
        }",
        r"namespace Foo {
          namespace Foo {
            export const Bar = 1;
            console.log(Foo.Bar);
          }
        }",
        r"interface Foo {
          bar: string;
          baz: Foo['bar'];
        }",
        r"type Foo = Array<Foo>;",
        r"import React from 'react';
        import { Fragment } from 'react';

        export const ComponentFoo = () => {
          return <div>Foo Foo</div>;
        };",
        r"import React from 'react';
        import { h } from 'some-other-jsx-lib';

        export const ComponentFoo = () => {
          return <div>Foo Foo</div>;
        };",
        r"import React from 'react';

        export const ComponentFoo = () => {
          return <div>Foo Foo</div>;
        };",
        r"declare module 'foo' {
          type Test = any;
          const x = 1;
          export = x;
        }",
        r"// not declared
        export namespace Foo {
          namespace Bar {
            namespace Baz {
              namespace Bam {
                const x = 1;
              }
            }
          }
        }",
        r"interface Foo {
          a: string;
        }
        interface Foo {
          b: Foo;
        }",
        r"let x = null;
        x = foo(x);",
        r"interface Foo {
          bar: string;
        }
        const Foo = 'bar';",
        r"let foo = 1;
        foo += 1;",
        r"interface Foo {
          bar: string;
        }
        type Bar = 1;
        export = Bar;",
        r"interface Foo {
          bar: string;
        }
        type Bar = 1;
        export = Foo;",
        r"namespace Foo {
          export const foo = 1;
        }
        export namespace Bar {
          import TheFoo = Foo;
        }",
    ];

    Tester::new(NoUnusedVars::NAME, pass, fail).test_and_snapshot();
}
