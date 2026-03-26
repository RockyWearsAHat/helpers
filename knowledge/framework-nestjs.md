# NestJS

## Overview

NestJS is an opinionated Node.js framework built on TypeScript. It borrows heavily from Angular — modules, decorators, dependency injection — and provides a structured architecture for building server-side applications. It runs on Express by default but can be switched to Fastify for better performance.

## Core Building Blocks

### Modules

Every NestJS app has a root module. Modules organize related features:

```typescript
@Module({
  imports: [DatabaseModule, AuthModule],
  controllers: [UsersController],
  providers: [UsersService, UsersRepository],
  exports: [UsersService], // available to other modules that import this one
})
export class UsersModule {}
```

| Property      | Purpose                                     |
| ------------- | ------------------------------------------- |
| `imports`     | Modules whose exported providers are needed |
| `controllers` | Controllers instantiated by this module     |
| `providers`   | Services, repositories, factories, helpers  |
| `exports`     | Providers available to importing modules    |

### Controllers

Handle incoming requests. Decorators define routes:

```typescript
@Controller("users")
export class UsersController {
  constructor(private readonly usersService: UsersService) {}

  @Get()
  findAll(@Query("page") page: number = 1): Promise<User[]> {
    return this.usersService.findAll(page);
  }

  @Get(":id")
  findOne(@Param("id", ParseIntPipe) id: number): Promise<User> {
    return this.usersService.findOne(id);
  }

  @Post()
  @HttpCode(201)
  @UsePipes(new ValidationPipe({ whitelist: true }))
  create(@Body() dto: CreateUserDto): Promise<User> {
    return this.usersService.create(dto);
  }

  @Put(":id")
  update(@Param("id", ParseIntPipe) id: number, @Body() dto: UpdateUserDto) {
    return this.usersService.update(id, dto);
  }

  @Delete(":id")
  @HttpCode(204)
  remove(@Param("id", ParseIntPipe) id: number) {
    return this.usersService.remove(id);
  }
}
```

### Providers (Services)

Any class with `@Injectable()` can be injected via constructor:

```typescript
@Injectable()
export class UsersService {
  constructor(
    @InjectRepository(User) private usersRepo: Repository<User>,
    private readonly cacheService: CacheService,
  ) {}

  async findOne(id: number): Promise<User> {
    const cached = await this.cacheService.get(`user:${id}`);
    if (cached) return cached;

    const user = await this.usersRepo.findOneBy({ id });
    if (!user) throw new NotFoundException(`User ${id} not found`);
    return user;
  }
}
```

## Dependency Injection

NestJS uses constructor-based DI. The IoC container manages lifecycle:

```typescript
// Custom provider with factory
@Module({
  providers: [
    {
      provide: 'DATABASE_CONNECTION',
      useFactory: async (config: ConfigService) => {
        return createConnection(config.get('DATABASE_URL'));
      },
      inject: [ConfigService],
    },
    {
      provide: UsersService,
      useClass: process.env.MOCK ? MockUsersService : UsersService,
    },
    {
      provide: 'API_KEY',
      useValue: process.env.API_KEY,
    },
  ],
})
```

### Scopes

| Scope                 | Lifecycle                  | Use Case             |
| --------------------- | -------------------------- | -------------------- |
| `DEFAULT` (singleton) | Shared across entire app   | Most services        |
| `REQUEST`             | New instance per request   | Request-scoped state |
| `TRANSIENT`           | New instance per injection | Stateful helpers     |

```typescript
@Injectable({ scope: Scope.REQUEST })
export class RequestContextService { ... }
```

## Pipes

Transform or validate input data:

```typescript
// Built-in ValidationPipe with class-validator
@Post()
@UsePipes(new ValidationPipe({
  whitelist: true,       // strip unknown properties
  forbidNonWhitelisted: true,  // throw on unknown properties
  transform: true,       // auto-transform to DTO type
}))
async create(@Body() dto: CreateUserDto) { ... }

// DTO with class-validator decorators
export class CreateUserDto {
  @IsString()
  @MinLength(2)
  @MaxLength(100)
  name: string;

  @IsEmail()
  email: string;

  @IsOptional()
  @IsEnum(UserRole)
  role?: UserRole;
}

// Custom pipe
@Injectable()
export class ParseDatePipe implements PipeTransform {
  transform(value: string): Date {
    const date = new Date(value);
    if (isNaN(date.getTime())) throw new BadRequestException('Invalid date');
    return date;
  }
}
```

## Guards

Authorization logic — return true/false to allow/deny:

```typescript
@Injectable()
export class JwtAuthGuard implements CanActivate {
  constructor(private jwtService: JwtService) {}

  async canActivate(context: ExecutionContext): Promise<boolean> {
    const request = context.switchToHttp().getRequest();
    const token = request.headers.authorization?.split(' ')[1];
    if (!token) throw new UnauthorizedException();

    try {
      const payload = await this.jwtService.verifyAsync(token);
      request.user = payload;
      return true;
    } catch {
      throw new UnauthorizedException();
    }
  }
}

@Injectable()
export class RolesGuard implements CanActivate {
  constructor(private reflector: Reflector) {}

  canActivate(context: ExecutionContext): boolean {
    const requiredRoles = this.reflector.get<string[]>('roles', context.getHandler());
    if (!requiredRoles) return true;
    const { user } = context.switchToHttp().getRequest();
    return requiredRoles.includes(user.role);
  }
}

// Usage
@UseGuards(JwtAuthGuard, RolesGuard)
@SetMetadata('roles', ['admin'])
@Delete(':id')
remove(@Param('id') id: string) { ... }
```

## Interceptors

Cross-cutting concerns — transform response, add logging, caching:

```typescript
@Injectable()
export class TransformInterceptor<T> implements NestInterceptor<
  T,
  Response<T>
> {
  intercept(
    context: ExecutionContext,
    next: CallHandler,
  ): Observable<Response<T>> {
    return next.handle().pipe(
      map((data) => ({
        data,
        statusCode: context.switchToHttp().getResponse().statusCode,
        timestamp: new Date().toISOString(),
      })),
    );
  }
}

@Injectable()
export class LoggingInterceptor implements NestInterceptor {
  intercept(context: ExecutionContext, next: CallHandler): Observable<any> {
    const now = Date.now();
    return next
      .handle()
      .pipe(
        tap(() =>
          console.log(`${context.getHandler().name}: ${Date.now() - now}ms`),
        ),
      );
  }
}
```

## Exception Filters

Customize error responses:

```typescript
@Catch(HttpException)
export class HttpExceptionFilter implements ExceptionFilter {
  catch(exception: HttpException, host: ArgumentsHost) {
    const ctx = host.switchToHttp();
    const response = ctx.getResponse();
    const status = exception.getStatus();

    response.status(status).json({
      statusCode: status,
      message: exception.message,
      timestamp: new Date().toISOString(),
      path: ctx.getRequest().url,
    });
  }
}

// Apply globally
app.useGlobalFilters(new HttpExceptionFilter());
```

## Request Lifecycle Order

```
Middleware → Guards → Interceptors (before) → Pipes → Handler → Interceptors (after) → Exception Filters
```

## TypeORM Integration

```typescript
@Module({
  imports: [
    TypeOrmModule.forRoot({
      type: 'postgres',
      host: 'localhost',
      port: 5432,
      database: 'mydb',
      entities: [User, Post],
      synchronize: false,  // NEVER true in production
    }),
    TypeOrmModule.forFeature([User]),
  ],
})
```

## Prisma Integration

```typescript
@Injectable()
export class PrismaService extends PrismaClient implements OnModuleInit {
  async onModuleInit() {
    await this.$connect();
  }
}

@Injectable()
export class UsersService {
  constructor(private prisma: PrismaService) {}

  findAll() {
    return this.prisma.user.findMany({ include: { posts: true } });
  }
}
```

## Microservices

NestJS supports multiple transport layers:

```typescript
// TCP microservice
const app = await NestFactory.createMicroservice<MicroserviceOptions>(AppModule, {
  transport: Transport.TCP,
  options: { host: '0.0.0.0', port: 3001 },
});

// Message patterns
@MessagePattern({ cmd: 'get_user' })
getUser(@Payload() data: { id: number }) {
  return this.usersService.findOne(data.id);
}

// Event patterns
@EventPattern('user_created')
handleUserCreated(@Payload() data: CreateUserEvent) {
  this.analyticsService.track(data);
}
```

Transport options: TCP, Redis, NATS, MQTT, gRPC, RabbitMQ, Kafka.

## GraphQL

```typescript
// Code-first approach
@ObjectType()
export class User {
  @Field(() => Int)
  id: number;

  @Field()
  name: string;

  @Field(() => [Post])
  posts: Post[];
}

@Resolver((of) => User)
export class UsersResolver {
  constructor(private usersService: UsersService) {}

  @Query((returns) => [User])
  users() {
    return this.usersService.findAll();
  }

  @Mutation((returns) => User)
  createUser(@Args("input") input: CreateUserInput) {
    return this.usersService.create(input);
  }

  @ResolveField()
  posts(@Parent() user: User) {
    return this.postsService.findByUser(user.id);
  }
}
```

## Task Scheduling & Queues

```typescript
// Cron jobs
@Injectable()
export class TasksService {
  @Cron('0 */5 * * * *')  // every 5 minutes
  handleCron() { ... }

  @Interval(60000)  // every 60 seconds
  handleInterval() { ... }
}

// Bull queues
@Processor('email')
export class EmailProcessor {
  @Process('welcome')
  async sendWelcome(job: Job<{ email: string }>) {
    await this.mailer.send(job.data.email, 'Welcome!');
  }
}

// Add to queue
await this.emailQueue.add('welcome', { email: user.email }, {
  delay: 5000,
  attempts: 3,
  backoff: { type: 'exponential', delay: 1000 },
});
```

## Configuration

```typescript
@Module({
  imports: [
    ConfigModule.forRoot({
      isGlobal: true,
      envFilePath: ['.env.local', '.env'],
      validationSchema: Joi.object({
        DATABASE_URL: Joi.string().required(),
        JWT_SECRET: Joi.string().required(),
        PORT: Joi.number().default(3000),
      }),
    }),
  ],
})
```

## Testing

```typescript
describe("UsersService", () => {
  let service: UsersService;
  let repo: MockType<Repository<User>>;

  beforeEach(async () => {
    const module = await Test.createTestingModule({
      providers: [
        UsersService,
        {
          provide: getRepositoryToken(User),
          useFactory: repositoryMockFactory,
        },
      ],
    }).compile();

    service = module.get(UsersService);
    repo = module.get(getRepositoryToken(User));
  });

  it("should find user by id", async () => {
    repo.findOneBy.mockResolvedValue({ id: 1, name: "Alice" });
    const result = await service.findOne(1);
    expect(result.name).toBe("Alice");
  });
});

// E2E test
describe("Users (e2e)", () => {
  let app: INestApplication;

  beforeAll(async () => {
    const module = await Test.createTestingModule({
      imports: [AppModule],
    }).compile();
    app = module.createNestApplication();
    await app.init();
  });

  it("GET /users", () => {
    return request(app.getHttpServer())
      .get("/users")
      .expect(200)
      .expect((res) => expect(res.body).toBeInstanceOf(Array));
  });
});
```

## Fastify Adapter

```typescript
import {
  FastifyAdapter,
  NestFastifyApplication,
} from "@nestjs/platform-fastify";

const app = await NestFactory.create<NestFastifyApplication>(
  AppModule,
  new FastifyAdapter(),
);
await app.listen(3000, "0.0.0.0");
```

Swap from Express to Fastify for ~2-3x throughput improvement. Most NestJS features work identically.

## OpenAPI

```typescript
const config = new DocumentBuilder()
  .setTitle("API")
  .setVersion("1.0")
  .addBearerAuth()
  .build();
const document = SwaggerModule.createDocument(app, config);
SwaggerModule.setup("docs", app, document);
```

Use `@ApiTags()`, `@ApiOperation()`, `@ApiResponse()`, `@ApiProperty()` decorators for documentation.
