# Objective-C Best Practices

## Objective-C Context

Objective-C is Apple's legacy language for macOS and iOS. While Swift is the modern choice, millions of lines of Objective-C exist in production apps, frameworks, and Apple's own SDKs. Understanding it is essential for maintaining existing codebases and bridging to Swift.

- **Message passing**: Objective-C uses Smalltalk-style messaging, not C++ method calls.
- **Dynamic runtime**: Methods are resolved at runtime. This enables powerful patterns (but also runtime crashes).
- **ARC (Automatic Reference Counting)**: Memory management without GC. The compiler inserts retain/release automatically.

## Classes and Messaging

```objc
// Header (.h)
@interface User : NSObject

@property (nonatomic, copy) NSString *name;
@property (nonatomic, assign) NSInteger age;
@property (nonatomic, copy, readonly) NSString *email;

- (instancetype)initWithName:(NSString *)name
                         age:(NSInteger)age
                       email:(NSString *)email;
- (NSString *)greeting;
+ (instancetype)userWithName:(NSString *)name;  // class method

@end

// Implementation (.m)
@implementation User

- (instancetype)initWithName:(NSString *)name
                         age:(NSInteger)age
                       email:(NSString *)email {
    self = [super init];
    if (self) {
        _name = [name copy];
        _age = age;
        _email = [email copy];
    }
    return self;
}

- (NSString *)greeting {
    return [NSString stringWithFormat:@"Hello, I'm %@, age %ld",
            self.name, (long)self.age];
}

+ (instancetype)userWithName:(NSString *)name {
    return [[self alloc] initWithName:name age:0 email:@""];
}

@end
```

## Properties and Memory

```objc
// Property attributes
@property (nonatomic, strong) NSArray *items;       // strong reference (default)
@property (nonatomic, weak) id<Delegate> delegate;  // weak (no retain cycle)
@property (nonatomic, copy) NSString *name;         // copy (for value semantics)
@property (nonatomic, assign) NSInteger count;      // assign (primitives)
@property (nonatomic, readonly) NSString *computed;  // no setter

// Avoid retain cycles in blocks
__weak typeof(self) weakSelf = self;
[self doAsyncWork:^{
    __strong typeof(weakSelf) strongSelf = weakSelf;
    if (!strongSelf) return;
    [strongSelf updateUI];
}];
```

## Protocols and Categories

```objc
// Protocol (like an interface)
@protocol Serializable <NSObject>

@required
- (NSDictionary *)toDictionary;

@optional
- (instancetype)initWithDictionary:(NSDictionary *)dict;

@end

// Conforming to a protocol
@interface User () <Serializable>
@end

@implementation User

- (NSDictionary *)toDictionary {
    return @{
        @"name": self.name ?: @"",
        @"age": @(self.age),
        @"email": self.email ?: @""
    };
}

@end

// Categories (add methods to existing classes)
@interface NSString (Validation)
- (BOOL)isValidEmail;
@end

@implementation NSString (Validation)
- (BOOL)isValidEmail {
    NSPredicate *pred = [NSPredicate predicateWithFormat:
        @"SELF MATCHES %@", @"[A-Z0-9a-z._%+-]+@[A-Za-z0-9.-]+\\.[A-Za-z]{2,}"];
    return [pred evaluateWithObject:self];
}
@end
```

## Blocks (Closures)

```objc
// Block syntax: ReturnType (^name)(Parameters)
typedef void (^CompletionHandler)(NSData * _Nullable data, NSError * _Nullable error);

- (void)fetchDataWithCompletion:(CompletionHandler)completion {
    dispatch_async(dispatch_get_global_queue(QOS_CLASS_DEFAULT, 0), ^{
        NSData *data = [self performFetch];
        dispatch_async(dispatch_get_main_queue(), ^{
            completion(data, nil);
        });
    });
}

// Inline block
[array enumerateObjectsUsingBlock:^(id obj, NSUInteger idx, BOOL *stop) {
    if ([obj isEqual:target]) {
        *stop = YES;
    }
}];
```

## Collections

```objc
// Literals (modern Objective-C)
NSArray *names = @[@"Alice", @"Bob", @"Charlie"];
NSDictionary *config = @{
    @"host": @"localhost",
    @"port": @8080,
    @"debug": @YES
};
NSNumber *num = @42;

// Mutable variants
NSMutableArray *items = [NSMutableArray array];
[items addObject:@"item"];

// Typed collections (lightweight generics)
NSArray<NSString *> *strings = @[@"a", @"b", @"c"];
NSDictionary<NSString *, NSNumber *> *scores = @{@"Alice": @95};

// Enumeration
for (NSString *name in names) {
    NSLog(@"%@", name);
}

// Filtering
NSPredicate *predicate = [NSPredicate predicateWithFormat:@"age >= 18"];
NSArray *adults = [users filteredArrayUsingPredicate:predicate];
```

## Bridging to Swift

```objc
// Nullability annotations (essential for clean Swift interop)
NS_ASSUME_NONNULL_BEGIN

@interface APIClient : NSObject

- (void)fetchUser:(NSString *)userId
       completion:(void (^)(User * _Nullable user, NSError * _Nullable error))completion;

@end

NS_ASSUME_NONNULL_END

// In Swift, this becomes:
// func fetchUser(_ userId: String, completion: @escaping (User?, Error?) -> Void)
```

## Key Rules

1. **Use ARC.** Never manually call `retain`, `release`, or `autorelease`. ARC handles it.
2. **Avoid retain cycles.** Use `weak` for delegates and `__weak`/`__strong` pattern in blocks.
3. **Copy NSString properties.** NSMutableString is a subclass — `copy` ensures immutability.
4. **Check for nil.** Messages to nil return 0/nil/NO (no crash), but logic bugs are silent.
5. **Add nullability annotations** (`_Nullable`, `_Nonnull`, `NS_ASSUME_NONNULL_BEGIN`) for Swift bridging.
6. **Prefer modern syntax.** Use literals (`@[]`, `@{}`, `@42`), generics (`NSArray<NSString *> *`), and property dot syntax.

---

*Sources: Apple Objective-C Programming Guide, iOS Programming (Big Nerd Ranch), Effective Objective-C 2.0 (Matt Galloway)*
