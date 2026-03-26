; ModuleID = 'bench_advanced.cpp'
source_filename = "bench_advanced.cpp"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

module asm ".globl _ZSt21ios_base_library_initv"

%"class.std::basic_ostream" = type { ptr, %"class.std::basic_ios" }
%"class.std::basic_ios" = type { %"class.std::ios_base", ptr, i8, i8, ptr, ptr, ptr, ptr }
%"class.std::ios_base" = type { ptr, i64, i64, i32, i32, i32, ptr, %"struct.std::ios_base::_Words", [8 x %"struct.std::ios_base::_Words"], i32, ptr, %"class.std::locale" }
%"struct.std::ios_base::_Words" = type { ptr, i64 }
%"class.std::locale" = type { ptr }
%"class.std::ctype" = type <{ %"class.std::locale::facet.base", [4 x i8], ptr, i8, [7 x i8], ptr, ptr, ptr, i8, [256 x i8], [256 x i8], i8, [6 x i8] }>
%"class.std::locale::facet.base" = type <{ ptr, i32 }>

@_ZSt4cout = external global %"class.std::basic_ostream", align 8
@.str = private unnamed_addr constant [3 x i8] c"  \00", align 1
@.str.1 = private unnamed_addr constant [3 x i8] c": \00", align 1
@.str.2 = private unnamed_addr constant [12 x i8] c"s  (result=\00", align 1
@.str.3 = private unnamed_addr constant [2 x i8] c")\00", align 1
@.str.4 = private unnamed_addr constant [66 x i8] c"=================================================================\00", align 1
@.str.5 = private unnamed_addr constant [61 x i8] c"  Agam vs Python vs Rust vs C++ \E2\80\94 Advanced Benchmark Suite\00", align 1
@.str.6 = private unnamed_addr constant [24 x i8] c"  C++ Runtime (GCC -O3)\00", align 1
@.str.7 = private unnamed_addr constant [4 x i8] c"Sum\00", align 1
@.str.8 = private unnamed_addr constant [10 x i8] c"Fibonacci\00", align 1
@.str.9 = private unnamed_addr constant [11 x i8] c"PrimeCount\00", align 1
@.str.10 = private unnamed_addr constant [7 x i8] c"MatMul\00", align 1
@.str.11 = private unnamed_addr constant [10 x i8] c"Integrate\00", align 1
@.str.12 = private unnamed_addr constant [19 x i8] c"  Total C++ time: \00", align 1
@.str.13 = private unnamed_addr constant [2 x i8] c"s\00", align 1

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local noundef i64 @_Z8sum_loopx(i64 noundef %0) #0 {
  %2 = icmp sgt i64 %0, 0
  br i1 %2, label %3, label %6

3:                                                ; preds = %1
  %4 = urem i64 %0, 7919
  %5 = add nuw nsw i64 %4, 1
  br label %8

6:                                                ; preds = %8, %1
  %7 = phi i64 [ 0, %1 ], [ %20, %8 ]
  ret i64 %7

8:                                                ; preds = %3, %8
  %9 = phi i64 [ %21, %8 ], [ 0, %3 ]
  %10 = phi i64 [ %16, %8 ], [ %5, %3 ]
  %11 = phi i64 [ %20, %8 ], [ 0, %3 ]
  %12 = mul nsw i64 %10, 57
  %13 = mul nsw i64 %9, 13
  %14 = add nsw i64 %12, 17
  %15 = add i64 %14, %13
  %16 = srem i64 %15, 1000003
  %17 = trunc i64 %16 to i32
  %18 = srem i32 %17, 1024
  %19 = sext i32 %18 to i64
  %20 = add nsw i64 %11, %19
  %21 = add nuw nsw i64 %9, 1
  %22 = icmp eq i64 %21, %0
  br i1 %22, label %6, label %8, !llvm.loop !5
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local noundef i64 @_Z9fibonaccix(i64 noundef %0) #0 {
  %2 = icmp sgt i64 %0, 0
  br i1 %2, label %3, label %20

3:                                                ; preds = %1
  %4 = and i64 %0, 7
  %5 = icmp ult i64 %0, 8
  br i1 %5, label %8, label %6

6:                                                ; preds = %3
  %7 = and i64 %0, 9223372036854775800
  br label %22

8:                                                ; preds = %22, %3
  %9 = phi i64 [ undef, %3 ], [ %32, %22 ]
  %10 = phi i64 [ 0, %3 ], [ %32, %22 ]
  %11 = phi i64 [ 1, %3 ], [ %33, %22 ]
  %12 = icmp eq i64 %4, 0
  br i1 %12, label %20, label %13

13:                                               ; preds = %8, %13
  %14 = phi i64 [ %15, %13 ], [ %10, %8 ]
  %15 = phi i64 [ %17, %13 ], [ %11, %8 ]
  %16 = phi i64 [ %18, %13 ], [ 0, %8 ]
  %17 = add nsw i64 %14, %15
  %18 = add i64 %16, 1
  %19 = icmp eq i64 %18, %4
  br i1 %19, label %20, label %13, !llvm.loop !7

20:                                               ; preds = %8, %13, %1
  %21 = phi i64 [ 0, %1 ], [ %9, %8 ], [ %15, %13 ]
  ret i64 %21

22:                                               ; preds = %22, %6
  %23 = phi i64 [ 0, %6 ], [ %32, %22 ]
  %24 = phi i64 [ 1, %6 ], [ %33, %22 ]
  %25 = phi i64 [ 0, %6 ], [ %34, %22 ]
  %26 = add nsw i64 %23, %24
  %27 = add nsw i64 %24, %26
  %28 = add nsw i64 %26, %27
  %29 = add nsw i64 %27, %28
  %30 = add nsw i64 %28, %29
  %31 = add nsw i64 %29, %30
  %32 = add nsw i64 %30, %31
  %33 = add nsw i64 %31, %32
  %34 = add i64 %25, 8
  %35 = icmp eq i64 %34, %7
  br i1 %35, label %8, label %22, !llvm.loop !9
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local noundef i64 @_Z12count_primesx(i64 noundef %0) #0 {
  %2 = icmp sgt i64 %0, 2
  br i1 %2, label %3, label %7

3:                                                ; preds = %1, %17
  %4 = phi i64 [ %20, %17 ], [ 2, %1 ]
  %5 = phi i64 [ %19, %17 ], [ 0, %1 ]
  %6 = icmp ult i64 %4, 4
  br i1 %6, label %17, label %13

7:                                                ; preds = %17, %1
  %8 = phi i64 [ 0, %1 ], [ %19, %17 ]
  ret i64 %8

9:                                                ; preds = %13
  %10 = add nuw nsw i64 %14, 1
  %11 = mul nsw i64 %10, %10
  %12 = icmp ugt i64 %11, %4
  br i1 %12, label %17, label %13, !llvm.loop !10

13:                                               ; preds = %3, %9
  %14 = phi i64 [ %10, %9 ], [ 2, %3 ]
  %15 = urem i64 %4, %14
  %16 = icmp eq i64 %15, 0
  br i1 %16, label %17, label %9

17:                                               ; preds = %9, %13, %3
  %18 = phi i64 [ 1, %3 ], [ 1, %9 ], [ 0, %13 ]
  %19 = add nuw nsw i64 %5, %18
  %20 = add nuw nsw i64 %4, 1
  %21 = icmp eq i64 %20, %0
  br i1 %21, label %7, label %3, !llvm.loop !11
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local noundef i64 @_Z15matrix_multiplyx(i64 noundef %0) #0 {
  %2 = icmp sgt i64 %0, 0
  br i1 %2, label %3, label %69

3:                                                ; preds = %1
  %4 = and i64 %0, 3
  %5 = icmp ult i64 %0, 4
  %6 = and i64 %0, 9223372036854775804
  %7 = icmp eq i64 %4, 0
  br label %8

8:                                                ; preds = %3, %66
  %9 = phi i64 [ %67, %66 ], [ 0, %3 ]
  %10 = phi i64 [ %63, %66 ], [ 0, %3 ]
  %11 = mul nsw i64 %9, %0
  br label %12

12:                                               ; preds = %61, %8
  %13 = phi i64 [ 0, %8 ], [ %64, %61 ]
  %14 = phi i64 [ %10, %8 ], [ %63, %61 ]
  br i1 %5, label %45, label %15

15:                                               ; preds = %12, %15
  %16 = phi i64 [ %42, %15 ], [ 0, %12 ]
  %17 = phi i64 [ %41, %15 ], [ 0, %12 ]
  %18 = phi i64 [ %43, %15 ], [ 0, %12 ]
  %19 = add nuw nsw i64 %16, %11
  %20 = mul nsw i64 %16, %0
  %21 = add nuw nsw i64 %20, %13
  %22 = mul nsw i64 %21, %19
  %23 = add nuw nsw i64 %22, %17
  %24 = or disjoint i64 %16, 1
  %25 = add nuw nsw i64 %24, %11
  %26 = mul nsw i64 %24, %0
  %27 = add nuw nsw i64 %26, %13
  %28 = mul nsw i64 %27, %25
  %29 = add nuw nsw i64 %28, %23
  %30 = or disjoint i64 %16, 2
  %31 = add nuw nsw i64 %30, %11
  %32 = mul nsw i64 %30, %0
  %33 = add nuw nsw i64 %32, %13
  %34 = mul nsw i64 %33, %31
  %35 = add nuw nsw i64 %34, %29
  %36 = or disjoint i64 %16, 3
  %37 = add nuw nsw i64 %36, %11
  %38 = mul nsw i64 %36, %0
  %39 = add nuw nsw i64 %38, %13
  %40 = mul nsw i64 %39, %37
  %41 = add nuw nsw i64 %40, %35
  %42 = add nuw nsw i64 %16, 4
  %43 = add i64 %18, 4
  %44 = icmp eq i64 %43, %6
  br i1 %44, label %45, label %15, !llvm.loop !12

45:                                               ; preds = %15, %12
  %46 = phi i64 [ undef, %12 ], [ %41, %15 ]
  %47 = phi i64 [ 0, %12 ], [ %42, %15 ]
  %48 = phi i64 [ 0, %12 ], [ %41, %15 ]
  br i1 %7, label %61, label %49

49:                                               ; preds = %45, %49
  %50 = phi i64 [ %58, %49 ], [ %47, %45 ]
  %51 = phi i64 [ %57, %49 ], [ %48, %45 ]
  %52 = phi i64 [ %59, %49 ], [ 0, %45 ]
  %53 = add nuw nsw i64 %50, %11
  %54 = mul nsw i64 %50, %0
  %55 = add nuw nsw i64 %54, %13
  %56 = mul nsw i64 %55, %53
  %57 = add nuw nsw i64 %56, %51
  %58 = add nuw nsw i64 %50, 1
  %59 = add i64 %52, 1
  %60 = icmp eq i64 %59, %4
  br i1 %60, label %61, label %49, !llvm.loop !13

61:                                               ; preds = %49, %45
  %62 = phi i64 [ %46, %45 ], [ %57, %49 ]
  %63 = add nsw i64 %62, %14
  %64 = add nuw nsw i64 %13, 1
  %65 = icmp eq i64 %64, %0
  br i1 %65, label %66, label %12, !llvm.loop !14

66:                                               ; preds = %61
  %67 = add nuw nsw i64 %9, 1
  %68 = icmp eq i64 %67, %0
  br i1 %68, label %69, label %8, !llvm.loop !15

69:                                               ; preds = %66, %1
  %70 = phi i64 [ 0, %1 ], [ %63, %66 ]
  ret i64 %70
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable
define dso_local noundef i64 @_Z12integrate_x2x(i64 noundef %0) #0 {
  %2 = icmp sgt i64 %0, 0
  br i1 %2, label %3, label %6

3:                                                ; preds = %1
  %4 = urem i64 %0, 1237
  %5 = add nuw nsw i64 %4, 3
  br label %8

6:                                                ; preds = %8, %1
  %7 = phi i64 [ 0, %1 ], [ %18, %8 ]
  ret i64 %7

8:                                                ; preds = %3, %8
  %9 = phi i64 [ %19, %8 ], [ 0, %3 ]
  %10 = phi i64 [ %14, %8 ], [ %5, %3 ]
  %11 = phi i64 [ %18, %8 ], [ 0, %3 ]
  %12 = mul nsw i64 %10, 73
  %13 = add nsw i64 %12, 19
  %14 = srem i64 %13, 65521
  %15 = mul nsw i64 %9, %9
  %16 = add nsw i64 %14, %15
  %17 = srem i64 %16, 4096
  %18 = add nsw i64 %17, %11
  %19 = add nuw nsw i64 %9, 1
  %20 = icmp eq i64 %19, %0
  br i1 %20, label %6, label %8, !llvm.loop !16
}

; Function Attrs: mustprogress uwtable
define dso_local noundef double @_Z9run_benchPKcPFxxEx(ptr noundef %0, ptr nocapture noundef readonly %1, i64 noundef %2) local_unnamed_addr #1 {
  %4 = tail call i64 @_ZNSt6chrono3_V212system_clock3nowEv() #7
  %5 = tail call noundef i64 %1(i64 noundef %2)
  %6 = tail call i64 @_ZNSt6chrono3_V212system_clock3nowEv() #7
  %7 = sub nsw i64 %6, %4
  %8 = sitofp i64 %7 to double
  %9 = fdiv double %8, 1.000000e+09
  %10 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull @.str, i64 noundef 2)
  %11 = icmp eq ptr %0, null
  br i1 %11, label %12, label %20

12:                                               ; preds = %3
  %13 = load ptr, ptr @_ZSt4cout, align 8, !tbaa !17
  %14 = getelementptr i8, ptr %13, i64 -24
  %15 = load i64, ptr %14, align 8
  %16 = getelementptr inbounds i8, ptr @_ZSt4cout, i64 %15
  %17 = getelementptr inbounds %"class.std::ios_base", ptr %16, i64 0, i32 5
  %18 = load i32, ptr %17, align 8, !tbaa !20
  %19 = or i32 %18, 1
  tail call void @_ZNSt9basic_iosIcSt11char_traitsIcEE5clearESt12_Ios_Iostate(ptr noundef nonnull align 8 dereferenceable(264) %16, i32 noundef %19)
  br label %23

20:                                               ; preds = %3
  %21 = tail call noundef i64 @strlen(ptr noundef nonnull dereferenceable(1) %0) #7
  %22 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull %0, i64 noundef %21)
  br label %23

23:                                               ; preds = %12, %20
  %24 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull @.str.1, i64 noundef 2)
  %25 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo9_M_insertIdEERSoT_(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, double noundef %9)
  %26 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) %25, ptr noundef nonnull @.str.2, i64 noundef 11)
  %27 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo9_M_insertIxEERSoT_(ptr noundef nonnull align 8 dereferenceable(8) %25, i64 noundef %5)
  %28 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) %27, ptr noundef nonnull @.str.3, i64 noundef 1)
  %29 = load ptr, ptr %27, align 8, !tbaa !17
  %30 = getelementptr i8, ptr %29, i64 -24
  %31 = load i64, ptr %30, align 8
  %32 = getelementptr inbounds i8, ptr %27, i64 %31
  %33 = getelementptr inbounds %"class.std::basic_ios", ptr %32, i64 0, i32 5
  %34 = load ptr, ptr %33, align 8, !tbaa !30
  %35 = icmp eq ptr %34, null
  br i1 %35, label %36, label %37

36:                                               ; preds = %23
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

37:                                               ; preds = %23
  %38 = getelementptr inbounds %"class.std::ctype", ptr %34, i64 0, i32 8
  %39 = load i8, ptr %38, align 8, !tbaa !33
  %40 = icmp eq i8 %39, 0
  br i1 %40, label %44, label %41

41:                                               ; preds = %37
  %42 = getelementptr inbounds %"class.std::ctype", ptr %34, i64 0, i32 9, i64 10
  %43 = load i8, ptr %42, align 1, !tbaa !36
  br label %49

44:                                               ; preds = %37
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %34)
  %45 = load ptr, ptr %34, align 8, !tbaa !17
  %46 = getelementptr inbounds ptr, ptr %45, i64 6
  %47 = load ptr, ptr %46, align 8
  %48 = tail call noundef signext i8 %47(ptr noundef nonnull align 8 dereferenceable(570) %34, i8 noundef signext 10)
  br label %49

49:                                               ; preds = %41, %44
  %50 = phi i8 [ %43, %41 ], [ %48, %44 ]
  %51 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) %27, i8 noundef signext %50)
  %52 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %51)
  ret double %9
}

; Function Attrs: nounwind
declare i64 @_ZNSt6chrono3_V212system_clock3nowEv() local_unnamed_addr #2

; Function Attrs: mustprogress norecurse uwtable
define dso_local noundef i32 @main(i32 noundef %0, ptr nocapture noundef readonly %1) local_unnamed_addr #3 {
  %3 = icmp sgt i32 %0, 1
  br i1 %3, label %4, label %28

4:                                                ; preds = %2
  %5 = getelementptr inbounds ptr, ptr %1, i64 1
  %6 = load ptr, ptr %5, align 8, !tbaa !37
  %7 = tail call i64 @__isoc23_strtoll(ptr noundef nonnull %6, ptr noundef null, i32 noundef 10) #7
  %8 = icmp eq i32 %0, 2
  br i1 %8, label %28, label %9

9:                                                ; preds = %4
  %10 = getelementptr inbounds ptr, ptr %1, i64 2
  %11 = load ptr, ptr %10, align 8, !tbaa !37
  %12 = tail call i64 @__isoc23_strtoll(ptr noundef nonnull %11, ptr noundef null, i32 noundef 10) #7
  %13 = icmp ugt i32 %0, 3
  br i1 %13, label %14, label %28

14:                                               ; preds = %9
  %15 = getelementptr inbounds ptr, ptr %1, i64 3
  %16 = load ptr, ptr %15, align 8, !tbaa !37
  %17 = tail call i64 @__isoc23_strtoll(ptr noundef nonnull %16, ptr noundef null, i32 noundef 10) #7
  %18 = icmp eq i32 %0, 4
  br i1 %18, label %28, label %19

19:                                               ; preds = %14
  %20 = getelementptr inbounds ptr, ptr %1, i64 4
  %21 = load ptr, ptr %20, align 8, !tbaa !37
  %22 = tail call i64 @__isoc23_strtoll(ptr noundef nonnull %21, ptr noundef null, i32 noundef 10) #7
  %23 = icmp ugt i32 %0, 5
  br i1 %23, label %24, label %28

24:                                               ; preds = %19
  %25 = getelementptr inbounds ptr, ptr %1, i64 5
  %26 = load ptr, ptr %25, align 8, !tbaa !37
  %27 = tail call i64 @__isoc23_strtoll(ptr noundef nonnull %26, ptr noundef null, i32 noundef 10) #7
  br label %28

28:                                               ; preds = %2, %4, %9, %14, %24, %19
  %29 = phi i64 [ %22, %24 ], [ %22, %19 ], [ 100, %14 ], [ 100, %9 ], [ 100, %4 ], [ 100, %2 ]
  %30 = phi i64 [ %12, %24 ], [ %12, %19 ], [ %12, %14 ], [ %12, %9 ], [ 40, %4 ], [ 40, %2 ]
  %31 = phi i64 [ %7, %24 ], [ %7, %19 ], [ %7, %14 ], [ %7, %9 ], [ %7, %4 ], [ 100000000, %2 ]
  %32 = phi i64 [ %17, %24 ], [ %17, %19 ], [ %17, %14 ], [ 100000, %9 ], [ 100000, %4 ], [ 100000, %2 ]
  %33 = phi i64 [ %27, %24 ], [ 10000000, %19 ], [ 10000000, %14 ], [ 10000000, %9 ], [ 10000000, %4 ], [ 10000000, %2 ]
  %34 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull @.str.4, i64 noundef 65)
  %35 = load ptr, ptr @_ZSt4cout, align 8, !tbaa !17
  %36 = getelementptr i8, ptr %35, i64 -24
  %37 = load i64, ptr %36, align 8
  %38 = getelementptr inbounds i8, ptr @_ZSt4cout, i64 %37
  %39 = getelementptr inbounds %"class.std::basic_ios", ptr %38, i64 0, i32 5
  %40 = load ptr, ptr %39, align 8, !tbaa !30
  %41 = icmp eq ptr %40, null
  br i1 %41, label %42, label %43

42:                                               ; preds = %28
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

43:                                               ; preds = %28
  %44 = getelementptr inbounds %"class.std::ctype", ptr %40, i64 0, i32 8
  %45 = load i8, ptr %44, align 8, !tbaa !33
  %46 = icmp eq i8 %45, 0
  br i1 %46, label %50, label %47

47:                                               ; preds = %43
  %48 = getelementptr inbounds %"class.std::ctype", ptr %40, i64 0, i32 9, i64 10
  %49 = load i8, ptr %48, align 1, !tbaa !36
  br label %55

50:                                               ; preds = %43
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %40)
  %51 = load ptr, ptr %40, align 8, !tbaa !17
  %52 = getelementptr inbounds ptr, ptr %51, i64 6
  %53 = load ptr, ptr %52, align 8
  %54 = tail call noundef signext i8 %53(ptr noundef nonnull align 8 dereferenceable(570) %40, i8 noundef signext 10)
  br label %55

55:                                               ; preds = %47, %50
  %56 = phi i8 [ %49, %47 ], [ %54, %50 ]
  %57 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, i8 noundef signext %56)
  %58 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %57)
  %59 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull @.str.5, i64 noundef 60)
  %60 = load ptr, ptr @_ZSt4cout, align 8, !tbaa !17
  %61 = getelementptr i8, ptr %60, i64 -24
  %62 = load i64, ptr %61, align 8
  %63 = getelementptr inbounds i8, ptr @_ZSt4cout, i64 %62
  %64 = getelementptr inbounds %"class.std::basic_ios", ptr %63, i64 0, i32 5
  %65 = load ptr, ptr %64, align 8, !tbaa !30
  %66 = icmp eq ptr %65, null
  br i1 %66, label %67, label %68

67:                                               ; preds = %55
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

68:                                               ; preds = %55
  %69 = getelementptr inbounds %"class.std::ctype", ptr %65, i64 0, i32 8
  %70 = load i8, ptr %69, align 8, !tbaa !33
  %71 = icmp eq i8 %70, 0
  br i1 %71, label %75, label %72

72:                                               ; preds = %68
  %73 = getelementptr inbounds %"class.std::ctype", ptr %65, i64 0, i32 9, i64 10
  %74 = load i8, ptr %73, align 1, !tbaa !36
  br label %80

75:                                               ; preds = %68
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %65)
  %76 = load ptr, ptr %65, align 8, !tbaa !17
  %77 = getelementptr inbounds ptr, ptr %76, i64 6
  %78 = load ptr, ptr %77, align 8
  %79 = tail call noundef signext i8 %78(ptr noundef nonnull align 8 dereferenceable(570) %65, i8 noundef signext 10)
  br label %80

80:                                               ; preds = %72, %75
  %81 = phi i8 [ %74, %72 ], [ %79, %75 ]
  %82 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, i8 noundef signext %81)
  %83 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %82)
  %84 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull @.str.6, i64 noundef 23)
  %85 = load ptr, ptr @_ZSt4cout, align 8, !tbaa !17
  %86 = getelementptr i8, ptr %85, i64 -24
  %87 = load i64, ptr %86, align 8
  %88 = getelementptr inbounds i8, ptr @_ZSt4cout, i64 %87
  %89 = getelementptr inbounds %"class.std::basic_ios", ptr %88, i64 0, i32 5
  %90 = load ptr, ptr %89, align 8, !tbaa !30
  %91 = icmp eq ptr %90, null
  br i1 %91, label %92, label %93

92:                                               ; preds = %80
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

93:                                               ; preds = %80
  %94 = getelementptr inbounds %"class.std::ctype", ptr %90, i64 0, i32 8
  %95 = load i8, ptr %94, align 8, !tbaa !33
  %96 = icmp eq i8 %95, 0
  br i1 %96, label %100, label %97

97:                                               ; preds = %93
  %98 = getelementptr inbounds %"class.std::ctype", ptr %90, i64 0, i32 9, i64 10
  %99 = load i8, ptr %98, align 1, !tbaa !36
  br label %105

100:                                              ; preds = %93
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %90)
  %101 = load ptr, ptr %90, align 8, !tbaa !17
  %102 = getelementptr inbounds ptr, ptr %101, i64 6
  %103 = load ptr, ptr %102, align 8
  %104 = tail call noundef signext i8 %103(ptr noundef nonnull align 8 dereferenceable(570) %90, i8 noundef signext 10)
  br label %105

105:                                              ; preds = %97, %100
  %106 = phi i8 [ %99, %97 ], [ %104, %100 ]
  %107 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, i8 noundef signext %106)
  %108 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %107)
  %109 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull @.str.4, i64 noundef 65)
  %110 = load ptr, ptr @_ZSt4cout, align 8, !tbaa !17
  %111 = getelementptr i8, ptr %110, i64 -24
  %112 = load i64, ptr %111, align 8
  %113 = getelementptr inbounds i8, ptr @_ZSt4cout, i64 %112
  %114 = getelementptr inbounds %"class.std::basic_ios", ptr %113, i64 0, i32 5
  %115 = load ptr, ptr %114, align 8, !tbaa !30
  %116 = icmp eq ptr %115, null
  br i1 %116, label %117, label %118

117:                                              ; preds = %105
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

118:                                              ; preds = %105
  %119 = getelementptr inbounds %"class.std::ctype", ptr %115, i64 0, i32 8
  %120 = load i8, ptr %119, align 8, !tbaa !33
  %121 = icmp eq i8 %120, 0
  br i1 %121, label %125, label %122

122:                                              ; preds = %118
  %123 = getelementptr inbounds %"class.std::ctype", ptr %115, i64 0, i32 9, i64 10
  %124 = load i8, ptr %123, align 1, !tbaa !36
  br label %130

125:                                              ; preds = %118
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %115)
  %126 = load ptr, ptr %115, align 8, !tbaa !17
  %127 = getelementptr inbounds ptr, ptr %126, i64 6
  %128 = load ptr, ptr %127, align 8
  %129 = tail call noundef signext i8 %128(ptr noundef nonnull align 8 dereferenceable(570) %115, i8 noundef signext 10)
  br label %130

130:                                              ; preds = %122, %125
  %131 = phi i8 [ %124, %122 ], [ %129, %125 ]
  %132 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, i8 noundef signext %131)
  %133 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %132)
  %134 = load ptr, ptr @_ZSt4cout, align 8, !tbaa !17
  %135 = getelementptr i8, ptr %134, i64 -24
  %136 = load i64, ptr %135, align 8
  %137 = getelementptr inbounds i8, ptr @_ZSt4cout, i64 %136
  %138 = getelementptr inbounds %"class.std::basic_ios", ptr %137, i64 0, i32 5
  %139 = load ptr, ptr %138, align 8, !tbaa !30
  %140 = icmp eq ptr %139, null
  br i1 %140, label %141, label %142

141:                                              ; preds = %130
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

142:                                              ; preds = %130
  %143 = getelementptr inbounds %"class.std::ctype", ptr %139, i64 0, i32 8
  %144 = load i8, ptr %143, align 8, !tbaa !33
  %145 = icmp eq i8 %144, 0
  br i1 %145, label %149, label %146

146:                                              ; preds = %142
  %147 = getelementptr inbounds %"class.std::ctype", ptr %139, i64 0, i32 9, i64 10
  %148 = load i8, ptr %147, align 1, !tbaa !36
  br label %154

149:                                              ; preds = %142
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %139)
  %150 = load ptr, ptr %139, align 8, !tbaa !17
  %151 = getelementptr inbounds ptr, ptr %150, i64 6
  %152 = load ptr, ptr %151, align 8
  %153 = tail call noundef signext i8 %152(ptr noundef nonnull align 8 dereferenceable(570) %139, i8 noundef signext 10)
  br label %154

154:                                              ; preds = %146, %149
  %155 = phi i8 [ %148, %146 ], [ %153, %149 ]
  %156 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, i8 noundef signext %155)
  %157 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %156)
  %158 = tail call noundef double @_Z9run_benchPKcPFxxEx(ptr noundef nonnull @.str.7, ptr noundef nonnull @_Z8sum_loopx, i64 noundef %31)
  %159 = fadd double %158, 0.000000e+00
  %160 = tail call noundef double @_Z9run_benchPKcPFxxEx(ptr noundef nonnull @.str.8, ptr noundef nonnull @_Z9fibonaccix, i64 noundef %30)
  %161 = fadd double %159, %160
  %162 = tail call noundef double @_Z9run_benchPKcPFxxEx(ptr noundef nonnull @.str.9, ptr noundef nonnull @_Z12count_primesx, i64 noundef %32)
  %163 = fadd double %161, %162
  %164 = tail call noundef double @_Z9run_benchPKcPFxxEx(ptr noundef nonnull @.str.10, ptr noundef nonnull @_Z15matrix_multiplyx, i64 noundef %29)
  %165 = fadd double %163, %164
  %166 = tail call noundef double @_Z9run_benchPKcPFxxEx(ptr noundef nonnull @.str.11, ptr noundef nonnull @_Z12integrate_x2x, i64 noundef %33)
  %167 = fadd double %165, %166
  %168 = load ptr, ptr @_ZSt4cout, align 8, !tbaa !17
  %169 = getelementptr i8, ptr %168, i64 -24
  %170 = load i64, ptr %169, align 8
  %171 = getelementptr inbounds i8, ptr @_ZSt4cout, i64 %170
  %172 = getelementptr inbounds %"class.std::basic_ios", ptr %171, i64 0, i32 5
  %173 = load ptr, ptr %172, align 8, !tbaa !30
  %174 = icmp eq ptr %173, null
  br i1 %174, label %175, label %176

175:                                              ; preds = %154
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

176:                                              ; preds = %154
  %177 = getelementptr inbounds %"class.std::ctype", ptr %173, i64 0, i32 8
  %178 = load i8, ptr %177, align 8, !tbaa !33
  %179 = icmp eq i8 %178, 0
  br i1 %179, label %183, label %180

180:                                              ; preds = %176
  %181 = getelementptr inbounds %"class.std::ctype", ptr %173, i64 0, i32 9, i64 10
  %182 = load i8, ptr %181, align 1, !tbaa !36
  br label %188

183:                                              ; preds = %176
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %173)
  %184 = load ptr, ptr %173, align 8, !tbaa !17
  %185 = getelementptr inbounds ptr, ptr %184, i64 6
  %186 = load ptr, ptr %185, align 8
  %187 = tail call noundef signext i8 %186(ptr noundef nonnull align 8 dereferenceable(570) %173, i8 noundef signext 10)
  br label %188

188:                                              ; preds = %180, %183
  %189 = phi i8 [ %182, %180 ], [ %187, %183 ]
  %190 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, i8 noundef signext %189)
  %191 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %190)
  %192 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull @.str.12, i64 noundef 18)
  %193 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo9_M_insertIdEERSoT_(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, double noundef %167)
  %194 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) %193, ptr noundef nonnull @.str.13, i64 noundef 1)
  %195 = load ptr, ptr %193, align 8, !tbaa !17
  %196 = getelementptr i8, ptr %195, i64 -24
  %197 = load i64, ptr %196, align 8
  %198 = getelementptr inbounds i8, ptr %193, i64 %197
  %199 = getelementptr inbounds %"class.std::basic_ios", ptr %198, i64 0, i32 5
  %200 = load ptr, ptr %199, align 8, !tbaa !30
  %201 = icmp eq ptr %200, null
  br i1 %201, label %202, label %203

202:                                              ; preds = %188
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

203:                                              ; preds = %188
  %204 = getelementptr inbounds %"class.std::ctype", ptr %200, i64 0, i32 8
  %205 = load i8, ptr %204, align 8, !tbaa !33
  %206 = icmp eq i8 %205, 0
  br i1 %206, label %210, label %207

207:                                              ; preds = %203
  %208 = getelementptr inbounds %"class.std::ctype", ptr %200, i64 0, i32 9, i64 10
  %209 = load i8, ptr %208, align 1, !tbaa !36
  br label %215

210:                                              ; preds = %203
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %200)
  %211 = load ptr, ptr %200, align 8, !tbaa !17
  %212 = getelementptr inbounds ptr, ptr %211, i64 6
  %213 = load ptr, ptr %212, align 8
  %214 = tail call noundef signext i8 %213(ptr noundef nonnull align 8 dereferenceable(570) %200, i8 noundef signext 10)
  br label %215

215:                                              ; preds = %207, %210
  %216 = phi i8 [ %209, %207 ], [ %214, %210 ]
  %217 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) %193, i8 noundef signext %216)
  %218 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %217)
  %219 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, ptr noundef nonnull @.str.4, i64 noundef 65)
  %220 = load ptr, ptr @_ZSt4cout, align 8, !tbaa !17
  %221 = getelementptr i8, ptr %220, i64 -24
  %222 = load i64, ptr %221, align 8
  %223 = getelementptr inbounds i8, ptr @_ZSt4cout, i64 %222
  %224 = getelementptr inbounds %"class.std::basic_ios", ptr %223, i64 0, i32 5
  %225 = load ptr, ptr %224, align 8, !tbaa !30
  %226 = icmp eq ptr %225, null
  br i1 %226, label %227, label %228

227:                                              ; preds = %215
  tail call void @_ZSt16__throw_bad_castv() #8
  unreachable

228:                                              ; preds = %215
  %229 = getelementptr inbounds %"class.std::ctype", ptr %225, i64 0, i32 8
  %230 = load i8, ptr %229, align 8, !tbaa !33
  %231 = icmp eq i8 %230, 0
  br i1 %231, label %235, label %232

232:                                              ; preds = %228
  %233 = getelementptr inbounds %"class.std::ctype", ptr %225, i64 0, i32 9, i64 10
  %234 = load i8, ptr %233, align 1, !tbaa !36
  br label %240

235:                                              ; preds = %228
  tail call void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570) %225)
  %236 = load ptr, ptr %225, align 8, !tbaa !17
  %237 = getelementptr inbounds ptr, ptr %236, i64 6
  %238 = load ptr, ptr %237, align 8
  %239 = tail call noundef signext i8 %238(ptr noundef nonnull align 8 dereferenceable(570) %225, i8 noundef signext 10)
  br label %240

240:                                              ; preds = %232, %235
  %241 = phi i8 [ %234, %232 ], [ %239, %235 ]
  %242 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8) @_ZSt4cout, i8 noundef signext %241)
  %243 = tail call noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8) %242)
  ret i32 0
}

; Function Attrs: nounwind
declare i64 @__isoc23_strtoll(ptr noundef, ptr noundef, i32 noundef) local_unnamed_addr #2

declare noundef nonnull align 8 dereferenceable(8) ptr @_ZSt16__ostream_insertIcSt11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_PKS3_l(ptr noundef nonnull align 8 dereferenceable(8), ptr noundef, i64 noundef) local_unnamed_addr #4

declare void @_ZNSt9basic_iosIcSt11char_traitsIcEE5clearESt12_Ios_Iostate(ptr noundef nonnull align 8 dereferenceable(264), i32 noundef) local_unnamed_addr #4

; Function Attrs: mustprogress nofree nounwind willreturn memory(argmem: read)
declare i64 @strlen(ptr nocapture noundef) local_unnamed_addr #5

declare noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo9_M_insertIdEERSoT_(ptr noundef nonnull align 8 dereferenceable(8), double noundef) local_unnamed_addr #4

declare noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo9_M_insertIxEERSoT_(ptr noundef nonnull align 8 dereferenceable(8), i64 noundef) local_unnamed_addr #4

declare noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo3putEc(ptr noundef nonnull align 8 dereferenceable(8), i8 noundef signext) local_unnamed_addr #4

declare noundef nonnull align 8 dereferenceable(8) ptr @_ZNSo5flushEv(ptr noundef nonnull align 8 dereferenceable(8)) local_unnamed_addr #4

; Function Attrs: noreturn
declare void @_ZSt16__throw_bad_castv() local_unnamed_addr #6

declare void @_ZNKSt5ctypeIcE13_M_widen_initEv(ptr noundef nonnull align 8 dereferenceable(570)) local_unnamed_addr #4

attributes #0 = { mustprogress nofree norecurse nosync nounwind willreturn memory(none) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { mustprogress uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nounwind "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { mustprogress norecurse uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #5 = { mustprogress nofree nounwind willreturn memory(argmem: read) "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #6 = { noreturn "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #7 = { nounwind }
attributes #8 = { noreturn }

!llvm.module.flags = !{!0, !1, !2, !3}
!llvm.ident = !{!4}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 8, !"PIC Level", i32 2}
!2 = !{i32 7, !"PIE Level", i32 2}
!3 = !{i32 7, !"uwtable", i32 2}
!4 = !{!"Ubuntu clang version 18.1.3 (1ubuntu1)"}
!5 = distinct !{!5, !6}
!6 = !{!"llvm.loop.mustprogress"}
!7 = distinct !{!7, !8}
!8 = !{!"llvm.loop.unroll.disable"}
!9 = distinct !{!9, !6}
!10 = distinct !{!10, !6}
!11 = distinct !{!11, !6}
!12 = distinct !{!12, !6}
!13 = distinct !{!13, !8}
!14 = distinct !{!14, !6}
!15 = distinct !{!15, !6}
!16 = distinct !{!16, !6}
!17 = !{!18, !18, i64 0}
!18 = !{!"vtable pointer", !19, i64 0}
!19 = !{!"Simple C++ TBAA"}
!20 = !{!21, !25, i64 32}
!21 = !{!"_ZTSSt8ios_base", !22, i64 8, !22, i64 16, !24, i64 24, !25, i64 28, !25, i64 32, !26, i64 40, !27, i64 48, !23, i64 64, !28, i64 192, !26, i64 200, !29, i64 208}
!22 = !{!"long", !23, i64 0}
!23 = !{!"omnipotent char", !19, i64 0}
!24 = !{!"_ZTSSt13_Ios_Fmtflags", !23, i64 0}
!25 = !{!"_ZTSSt12_Ios_Iostate", !23, i64 0}
!26 = !{!"any pointer", !23, i64 0}
!27 = !{!"_ZTSNSt8ios_base6_WordsE", !26, i64 0, !22, i64 8}
!28 = !{!"int", !23, i64 0}
!29 = !{!"_ZTSSt6locale", !26, i64 0}
!30 = !{!31, !26, i64 240}
!31 = !{!"_ZTSSt9basic_iosIcSt11char_traitsIcEE", !21, i64 0, !26, i64 216, !23, i64 224, !32, i64 225, !26, i64 232, !26, i64 240, !26, i64 248, !26, i64 256}
!32 = !{!"bool", !23, i64 0}
!33 = !{!34, !23, i64 56}
!34 = !{!"_ZTSSt5ctypeIcE", !35, i64 0, !26, i64 16, !32, i64 24, !26, i64 32, !26, i64 40, !26, i64 48, !23, i64 56, !23, i64 57, !23, i64 313, !23, i64 569}
!35 = !{!"_ZTSNSt6locale5facetE", !28, i64 8}
!36 = !{!23, !23, i64 0}
!37 = !{!26, !26, i64 0}
