declare float @llvm.minimumnum.f32(float, float)
declare double @llvm.minimumnum.f64(double, double)
declare <8 x float> @llvm.minimumnum.v8f32(<8 x float>, <8 x float>)
declare float @llvm.maximumnum.f32(float, float)

define float @f32_nsz(float %a, float %b) {
  %r = call nsz float @llvm.minimumnum.f32(float %a, float %b)
  ret float %r
}
define float @f32_det(float %a, float %b) {
  %r = call float @llvm.minimumnum.f32(float %a, float %b)
  ret float %r
}
define double @f64_nsz(double %a, double %b) {
  %r = call nsz double @llvm.minimumnum.f64(double %a, double %b)
  ret double %r
}
define double @f64_det(double %a, double %b) {
  %r = call double @llvm.minimumnum.f64(double %a, double %b)
  ret double %r
}
define <8 x float> @v8f32_nsz(<8 x float> %a, <8 x float> %b) {
  %r = call nsz <8 x float> @llvm.minimumnum.v8f32(<8 x float> %a, <8 x float> %b)
  ret <8 x float> %r
}
define <8 x float> @v8f32_det(<8 x float> %a, <8 x float> %b) {
  %r = call <8 x float> @llvm.minimumnum.v8f32(<8 x float> %a, <8 x float> %b)
  ret <8 x float> %r
}
define float @max_f32_nsz(float %a, float %b) {
  %r = call nsz float @llvm.maximumnum.f32(float %a, float %b)
  ret float %r
}
define float @max_f32_det(float %a, float %b) {
  %r = call float @llvm.maximumnum.f32(float %a, float %b)
  ret float %r
}
